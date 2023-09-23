pub mod utils;

use std::ffi::c_char;
use std::os::fd::AsRawFd;
use std::ptr::null_mut;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use bevy::utils::HashSet;
use bevy::{
    ecs::entity::EntityMap,
    prelude::*,
    render::{
        render_asset::{PrepareAssetSet, RenderAssets},
        renderer::{RenderDevice, RenderQueue},
        texture::{DefaultImageSampler, GpuImage},
        Extract, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use drm::control::{crtc, framebuffer};
use drm_fourcc::DrmFormat;
use drm_fourcc::DrmFourcc;
use drm_fourcc::DrmModifier;
use glow::HasContext;
use glow::NativeFramebuffer;
use khronos_egl::Attrib;
use khronos_egl::Boolean;
use khronos_egl::EGLClientBuffer;
use khronos_egl::EGLContext;
use khronos_egl::EGLDisplay;
use khronos_egl::EGLImage;
use khronos_egl::Enum;
use khronos_egl::Int;
use tracing::span;
use tracing::Level;
use wgpu::Extent3d;
use wgpu::Texture;
use wgpu::TextureDimension;
use wgpu::TextureUsages;
use wgpu::{TextureFormat, TextureViewDescriptor};
use wgpu_hal::{api::Gles, MemoryFlags, TextureUses};

use crate::drm::connectors::Connector;
use crate::gbm::buffer::GbmBuffer;
use crate::{
    drm::{
        surface::{drm_framebuffer_descriptor, DrmSurface},
        DrmDevice,
    },
    gbm::GbmDevice,
};

use self::utils::call_egl_double_vec;
use self::utils::call_egl_vec;
use self::utils::get_egl_extensions;

pub struct TtyRenderFunctions {
    pub egl_create_image_khr: unsafe extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    pub gl_eglimage_target_renderbuffer_storage_oes: unsafe extern "system" fn(Enum, EGLImage),
    pub egl_query_dma_buf_modifiers_ext:
        extern "system" fn(EGLDisplay, Int, Int, *mut u64, *mut Boolean, *mut Int) -> Boolean,
    pub egl_query_dmabuf_format_ext:
        extern "system" fn(EGLDisplay, Int, *mut u32, *mut Int) -> Boolean,
}
impl TtyRenderFunctions {
    pub fn new(egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>) -> Result<Self> {
        Ok(Self {
            egl_create_image_khr: unsafe {
                std::mem::transmute(
                    egl.get_proc_address("eglCreateImageKHR")
                        .ok_or_else(|| anyhow!("gl function eglCreateImageKHR not exists"))?,
                )
            },
            gl_eglimage_target_renderbuffer_storage_oes: unsafe {
                std::mem::transmute(
                    egl.get_proc_address("glEGLImageTargetRenderbufferStorageOES")
                        .ok_or_else(|| {
                            anyhow!("gl function glEGLImageTargetRenderbufferStorageOES not exists")
                        })?,
                )
            },
            egl_query_dma_buf_modifiers_ext: unsafe {
                std::mem::transmute(
                    egl.get_proc_address("eglQueryDmaBufModifiersEXT")
                        .ok_or_else(|| {
                            anyhow!("gl function eglQueryDmaBufModifiersEXT not exists")
                        })?,
                )
            },
            egl_query_dmabuf_format_ext: unsafe {
                std::mem::transmute(
                    egl.get_proc_address("eglQueryDmaBufFormatsEXT")
                        .ok_or_else(|| {
                            anyhow!("gl function eglQueryDmaBufFormatsEXT not exists")
                        })?,
                )
            },
        })
    }
}

#[derive(Resource, Default)]
pub struct TtyRenderState {
    pub buffers: HashMap<framebuffer::Handle, GpuImage>,
    pub entity_map: HashMap<Entity, Entity>,
    pub functions: Option<TtyRenderFunctions>,
    pub formats: Option<Vec<DrmFormat>>,
}

impl TtyRenderState {
    pub fn get_formats(&mut self, render_device: &wgpu::Device) -> Result<&[DrmFormat]> {
        if self.formats.is_some() {
            return Ok(&**self.formats.as_ref().unwrap());
        }
        unsafe {
            render_device.as_hal::<Gles, _, _>(|hal_device| {
                let hal_device = hal_device.ok_or_else(|| anyhow!("gpu backend is not egl"))?;
                let egl_context = hal_device.context();
                let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
                    .egl_instance()
                    .ok_or_else(|| anyhow!("gpu backend is not egl"))?;
                let egl_display = egl_context
                    .raw_display()
                    .ok_or_else(|| anyhow!("egl display is not valid"))?;

                let functions = self
                    .functions
                    .get_or_insert_with(|| TtyRenderFunctions::new(egl).unwrap());

                let extensions = get_egl_extensions(egl, *egl_display)?;
                let fourcc_list = if !extensions.contains("EGL_EXT_image_dma_buf_import_modifiers")
                {
                    vec![DrmFourcc::Argb8888, DrmFourcc::Xrgb8888]
                } else {
                    call_egl_vec(egl, |num, vec, p_num| {
                        (functions.egl_query_dmabuf_format_ext)(
                            egl_display.as_ptr(),
                            num,
                            vec,
                            p_num,
                        )
                    })?
                    .into_iter()
                    .filter_map(|f| DrmFourcc::try_from(f).ok())
                    .collect()
                };

                let mut render_formats = HashSet::new();
                for fourcc in fourcc_list.iter().cloned() {
                    let (mods, external) = call_egl_double_vec(egl, |num, vec1, vec2, p_num| {
                        (functions.egl_query_dma_buf_modifiers_ext)(
                            egl_display.as_ptr(),
                            fourcc as i32,
                            num,
                            vec1,
                            vec2,
                            p_num,
                        )
                    })
                    .map_err(|e| anyhow!("egl error: {e}"))?;
                    if mods.len() == 0 {
                        render_formats.insert(DrmFormat {
                            code: fourcc,
                            modifier: DrmModifier::Invalid,
                        });
                    }
                    for (modifier, external_only) in mods.into_iter().zip(external.into_iter()) {
                        if external_only == 0 {
                            render_formats.insert(DrmFormat {
                                code: fourcc,
                                modifier: DrmModifier::from(modifier),
                            });
                        }
                    }
                }

                self.formats = Some(render_formats.into_iter().collect());
                Result::<_, anyhow::Error>::Ok(&**self.formats.as_ref().unwrap())
            })
        }
    }
}

pub struct TtyRenderPlugin;
impl Plugin for TtyRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TtyRenderState>()
                .add_system(extract_drm_surfaces.in_schedule(ExtractSchedule))
                .add_system(prepare_drm_surface.in_set(PrepareAssetSet::PostAssetPrepare))
                .add_system(commit.after(RenderSet::Render).before(RenderSet::Cleanup));
        }
    }
}

#[tracing::instrument(skip_all)]
pub fn extract_drm_surfaces(
    surface_query: Extract<Query<(&DrmSurface, &Connector, &Parent)>>,
    drm_query: Extract<Query<(Entity, &DrmDevice, &GbmDevice)>>,
    mut state: ResMut<TtyRenderState>,
    mut commands: Commands,
) {
    drm_query.for_each(|(entity, drm_device, gbm_device)| {
        let render_entity = commands
            .spawn((drm_device.clone(), gbm_device.clone()))
            .id();
        state.entity_map.insert(entity, render_entity);
    });
    surface_query.for_each(|(surface, conn, parent)| {
        let mut entity_command = commands.spawn((surface.clone(), conn.clone()));
        if let Some(parent_entity) = state.entity_map.get(&parent.get()) {
            entity_command.set_parent(*parent_entity);
        }
    });
}

#[tracing::instrument(skip_all)]
pub fn prepare_drm_surface(
    mut state: ResMut<TtyRenderState>,
    surface_query: Query<(&DrmSurface, &Parent)>,
    drm_query: Query<(&DrmDevice, &GbmDevice)>,
    mut render_images: ResMut<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    default_sampler: Res<DefaultImageSampler>,
) {
    surface_query.for_each(|(surface, parent)| {
        let Ok((drm, gbm)) = drm_query.get(parent.get()) else {
            return;
        };
        let _span =
            span!(Level::ERROR,"prepare drm buffer",device=%drm.path.to_string_lossy()).entered();

        let Ok(formats) = state
            .get_formats(render_device.wgpu_device())
            .map_err(|e| error!("failed to get gl formats: {e}"))
        else {
            return;
        };

        let mut surface_guard = surface.inner.lock().unwrap();
        let Ok(buffer) = surface_guard
            .get_buffer(drm, gbm, formats)
            .map_err(|e| error!("failed to create gbm buffer: {}", e))
        else {
            return;
        };

        let gpu_image = if let Some(gpu_image) = state.buffers.get(&buffer.framebuffer) {
            gpu_image.clone()
        } else {
            let Ok(texture) =
                create_framebuffer_texture(&mut state, &render_device.wgpu_device(), &buffer)
                    .map_err(|e| error!("failed to bind gbm buffer: {e}"))
            else {
                return;
            };
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            let gpu_image = GpuImage {
                texture: texture.into(),
                texture_view: texture_view.into(),
                texture_format: TextureFormat::Bgra8UnormSrgb,
                sampler: (**default_sampler).clone(),
                size: buffer.size.as_vec2(),
                mip_level_count: 1,
            };
            info!("create drm image");
            state.buffers.insert(buffer.framebuffer, gpu_image.clone());
            gpu_image
        };
        render_images.insert(surface.image.clone(), gpu_image);
    });
}

pub const LINUX_DRM_FOURCC_EXT: u32 = 0x3271;

#[tracing::instrument(skip_all)]
pub fn create_framebuffer_texture(
    state: &mut TtyRenderState,
    render_device: &wgpu::Device,
    buffer: &GbmBuffer,
) -> Result<Texture> {
    unsafe {
        let hal_texture = render_device.as_hal::<Gles, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| anyhow!("gpu backend is not egl"))?;
            let egl_context = hal_device.context();
            let gl: &glow::Context = &egl_context.lock();
            let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
                .egl_instance()
                .ok_or_else(|| anyhow!("gpu backend is not egl"))?;
            let egl_display = egl_context
                .raw_display()
                .ok_or_else(|| anyhow!("egl display is not valid"))?;

            let functions = state
                .functions
                .get_or_insert_with(|| TtyRenderFunctions::new(egl).unwrap());
            let renderbuffer = do_create_renderbuffer(gl, buffer, egl_display.as_ptr(), functions)?;

            let hal_texture = hal_device.texture_from_raw_renderbuffer(
                renderbuffer.0,
                &wgpu_hal::TextureDescriptor {
                    label: Some("gbm renderbuffer"),
                    size: Extent3d {
                        width: buffer.size.x as u32,
                        height: buffer.size.y as u32,
                        depth_or_array_layers: 1,
                        ..default()
                    },
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Bgra8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUses::COLOR_TARGET
                        | TextureUses::DEPTH_STENCIL_READ
                        | TextureUses::DEPTH_STENCIL_WRITE,
                    view_formats: vec![],
                    memory_flags: MemoryFlags::empty(),
                },
                None,
            );

            Result::<_, anyhow::Error>::Ok(hal_texture)
        })?;
        let texture = render_device
            .create_texture_from_hal::<Gles>(hal_texture, &drm_framebuffer_descriptor(buffer.size));
        Ok(texture)
    }
}

pub const DMA_BUF_PLANE0_FD_EXT: u32 = 0x3272;
pub const DMA_BUF_PLANE0_OFFSET_EXT: u32 = 0x3273;
pub const DMA_BUF_PLANE0_PITCH_EXT: u32 = 0x3274;
pub const DMA_BUF_PLANE0_MODIFIER_LO_EXT: u32 = 0x3443;
pub const DMA_BUF_PLANE0_MODIFIER_HI_EXT: u32 = 0x3444;

pub const DMA_BUF_PLANE1_FD_EXT: u32 = 0x3275;
pub const DMA_BUF_PLANE1_OFFSET_EXT: u32 = 0x3276;
pub const DMA_BUF_PLANE1_PITCH_EXT: u32 = 0x3277;
pub const DMA_BUF_PLANE1_MODIFIER_LO_EXT: u32 = 0x3445;
pub const DMA_BUF_PLANE1_MODIFIER_HI_EXT: u32 = 0x3446;

pub const DMA_BUF_PLANE2_FD_EXT: u32 = 0x3278;
pub const DMA_BUF_PLANE2_OFFSET_EXT: u32 = 0x3279;
pub const DMA_BUF_PLANE2_PITCH_EXT: u32 = 0x327A;
pub const DMA_BUF_PLANE2_MODIFIER_LO_EXT: u32 = 0x3447;
pub const DMA_BUF_PLANE2_MODIFIER_HI_EXT: u32 = 0x3448;

pub const DMA_BUF_PLANE3_FD_EXT: u32 = 0x3440;
pub const DMA_BUF_PLANE3_OFFSET_EXT: u32 = 0x3441;
pub const DMA_BUF_PLANE3_PITCH_EXT: u32 = 0x3442;
pub const DMA_BUF_PLANE3_MODIFIER_LO_EXT: u32 = 0x3449;
pub const DMA_BUF_PLANE3_MODIFIER_HI_EXT: u32 = 0x344A;

pub const LINUX_DMA_BUF_EXT: u32 = 0x3270;

const PLANE_ATTR_NAMES: [(u32, u32, u32, u32, u32); 4] = [
    (
        DMA_BUF_PLANE0_FD_EXT,
        DMA_BUF_PLANE0_OFFSET_EXT,
        DMA_BUF_PLANE0_PITCH_EXT,
        DMA_BUF_PLANE0_MODIFIER_LO_EXT,
        DMA_BUF_PLANE0_MODIFIER_HI_EXT,
    ),
    (
        DMA_BUF_PLANE1_FD_EXT,
        DMA_BUF_PLANE1_OFFSET_EXT,
        DMA_BUF_PLANE1_PITCH_EXT,
        DMA_BUF_PLANE1_MODIFIER_LO_EXT,
        DMA_BUF_PLANE1_MODIFIER_HI_EXT,
    ),
    (
        DMA_BUF_PLANE2_FD_EXT,
        DMA_BUF_PLANE2_OFFSET_EXT,
        DMA_BUF_PLANE2_PITCH_EXT,
        DMA_BUF_PLANE2_MODIFIER_LO_EXT,
        DMA_BUF_PLANE2_MODIFIER_HI_EXT,
    ),
    (
        DMA_BUF_PLANE3_FD_EXT,
        DMA_BUF_PLANE3_OFFSET_EXT,
        DMA_BUF_PLANE3_PITCH_EXT,
        DMA_BUF_PLANE3_MODIFIER_LO_EXT,
        DMA_BUF_PLANE3_MODIFIER_HI_EXT,
    ),
];

unsafe fn do_create_renderbuffer(
    gl: &glow::Context,
    buffer: &GbmBuffer,
    display: EGLDisplay,
    functions: &TtyRenderFunctions,
) -> Result<glow::Renderbuffer> {
    debug!("gbm buffer: {buffer:?}");

    let mut request = vec![
        khronos_egl::WIDTH,
        buffer.size.x,
        khronos_egl::HEIGHT,
        buffer.size.y,
        LINUX_DRM_FOURCC_EXT as i32,
        buffer.format as i32,
    ];
    for (i, plane) in buffer.planes.iter().enumerate() {
        request.extend([
            PLANE_ATTR_NAMES[i].0 as i32,
            plane.fd.as_raw_fd(),
            PLANE_ATTR_NAMES[i].1 as i32,
            plane.offset as i32,
            PLANE_ATTR_NAMES[i].2 as i32,
            plane.stride as i32,
        ]);
        if buffer.modifier != DrmModifier::Invalid && buffer.modifier != DrmModifier::Linear {
            request.extend([
                PLANE_ATTR_NAMES[i].3 as i32,
                u64::from(buffer.modifier) as i32,
                PLANE_ATTR_NAMES[i].4 as i32,
                (u64::from(buffer.modifier) >> 32) as u32 as i32,
            ])
        }
    }
    request.push(khronos_egl::NONE);
    debug!("eglCreateImageKHR({request:?})");

    let image = unsafe {
        (functions.egl_create_image_khr)(
            display,
            khronos_egl::NO_CONTEXT,
            LINUX_DMA_BUF_EXT,
            std::ptr::null_mut(),
            request.as_ptr(),
        )
    };
    if image == null_mut() {
        bail!("failed to create EGLImage");
    }

    let renderbuffer = gl
        .create_renderbuffer()
        .map_err(|e| anyhow!("failed to create gl renderbuffer: {}", e))?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(renderbuffer));
    (functions.gl_eglimage_target_renderbuffer_storage_oes)(glow::RENDERBUFFER, image);
    let error = gl.get_error();
    if error != 0 {
        bail!("gl error: EGLImageTargetRenderbufferStorageOES: {error}");
    }

    let framebuffer = gl
        .create_framebuffer()
        .map_err(|e| anyhow!("failed to create framebuffer: {e}"))?;
    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
    gl.framebuffer_renderbuffer(
        glow::FRAMEBUFFER,
        glow::COLOR_ATTACHMENT0,
        glow::RENDERBUFFER,
        Some(renderbuffer),
    );
    gl.clear_color(1.0, 1.0, 1.0, 1.0); // TODO remove
    gl.clear(glow::COLOR_BUFFER_BIT); // TODO remove
    gl.bind_renderbuffer(glow::RENDERBUFFER, None);
    gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    gl.delete_framebuffer(framebuffer);
    Ok(renderbuffer)
}

#[tracing::instrument(skip_all)]
pub fn commit(
    surface_query: Query<(&DrmSurface, &Connector, &Parent)>,
    drm_query: Query<&DrmDevice>,
) {
    surface_query.for_each(|(surface, conn, parent)| {
        let Ok(drm) = drm_query.get(parent.get()) else {
            return;
        };
        let _span =
            span!(Level::ERROR,"commit drm buffer",device=%drm.path.to_string_lossy()).entered();
        if let Err(e) = surface.commit(&conn, drm) {
            error!("failed to commit surface to drm: {e}");
        };
        debug!("commmit drm render buffer");
    });
}
