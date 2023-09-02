use crate::{
    prelude::*,
    util::rect::IRect,
    wl::{
        buffer::{EGLBuffer, UninitedWlBuffer, WlMemoryBuffer},
        surface::WlSurface,
    },
    zwp::dmabufparam::DmaBuffer,
};

use drm_fourcc::DrmModifier;
use thiserror::Error;
use wayland_backend::server::WeakHandle;

use std::{
    collections::{HashMap, HashSet},
    ffi::{c_int, c_uint, c_void},
    num::NonZeroU32,
    os::fd::AsRawFd,
    ptr::null_mut,
};

use bevy::{
    prelude::{info, Vec2},
    render::texture::GpuImage,
};
use glow::{HasContext, NativeRenderbuffer, NativeTexture};
pub const LINUX_DMA_BUF_EXT: u32 = 0x3270;
pub const WAYLAND_PLANE_WL: c_uint = 0x31D6;
pub const LINUX_DRM_FOURCC_EXT: u32 = 0x3271;

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

pub const EGL_NO_IMAGE_KHR: *mut c_void = null_mut();

use khronos_egl::{Boolean, EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, Enum, Int, NONE};
use wgpu::{FilterMode, SamplerDescriptor, Texture, TextureAspect};
use wgpu_hal::Api;
use wgpu_hal::{api::Gles, MemoryFlags, TextureUses};

use super::importnode::DWayDisplayHandles;

#[derive(Error, Debug)]
pub enum ImportSurfaceError {
    #[error("gl function `{0}` not exists")]
    FunctionNotExists(String),
    #[error("no opengl display available")]
    DisplayNotAvailable,
    #[error("failed to get hal device")]
    FailedToGetHal,
    #[error("failed to import dma buffer")]
    FailedToImportDmaBuffer,
    #[error("failed to import egl buffer")]
    FailedToImportEglBuffer,
    #[error("gpu backend is not egl")]
    BackendIsNotEGL,
    #[error("failed to create dma image")]
    FailedToCreateDmaImage,
    #[error("failed to create texture: {0}")]
    FailedToCreateSurface(String),
    #[error("failed to create render buffer: {0}")]
    FailedToCreateRenderBuffer(String),
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(wl_shm::Format),
    #[error("egl error: {0:?}")]
    EglError(#[from] khronos_egl::Error),
    #[error("{0}")]
    Unknown(#[from] anyhow::Error),
}
use ImportSurfaceError::*;

pub struct EglState {
    pub egl_create_image_khr: unsafe extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    pub gl_eglimage_target_texture2_does:
        unsafe extern "system" fn(target: Enum, image: *const c_void),
    pub egl_bind_wayland_display_wl: unsafe extern "system" fn(EGLDisplay, *mut c_void) -> Boolean,
    pub egl_unbind_wayland_display_wl:
        unsafe extern "system" fn(EGLDisplay, *mut c_void) -> Boolean,
    pub extensions: HashSet<String>,
    pub wayland_map: HashMap<Entity, WeakHandle>,
}
impl EglState {
    pub fn bind_wayland(
        &mut self,
        wayland_map: &HashMap<Entity, DisplayHandle>,
        egl_display: khronos_egl::Display,
    ) {
        for (entity, handle) in wayland_map {
            if !self.wayland_map.contains_key(entity) {
                let ptr = handle.backend_handle().display_ptr();
                unsafe { (self.egl_bind_wayland_display_wl)(egl_display.as_ptr(), ptr as *mut _) };
                self.wayland_map
                    .insert(*entity, handle.backend_handle().downgrade());
            }
        }
        self.wayland_map.retain(|entity, handle| {
            if !wayland_map.contains_key(entity) {
                if let Some(handle) = handle.upgrade() {
                    let ptr = handle.display_ptr();
                    unsafe {
                        (self.egl_bind_wayland_display_wl)(egl_display.as_ptr(), ptr as *mut _)
                    };
                }
                false
            } else {
                true
            }
        });
    }
    pub fn new(
        gl: &glow::Context,
        egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>,
    ) -> Result<Self> {
        let extensions = gl.supported_extensions().clone();
        unsafe {
            let egl_create_image_khr = std::mem::transmute(
                egl.get_proc_address("eglCreateImageKHR")
                    .ok_or_else(|| FunctionNotExists("eglCreateImageKHR".into()))?,
            );
            let gl_eglimage_target_texture2_does = std::mem::transmute(
                egl.get_proc_address("glEGLImageTargetTexture2DOES")
                    .ok_or_else(|| FunctionNotExists("glEGLImageTargetTexture2DOES".into()))?,
            );
            let egl_bind_wayland_display_wl = std::mem::transmute(
                egl.get_proc_address("eglBindWaylandDisplayWL")
                    .ok_or_else(|| FunctionNotExists("eglBindWaylandDisplayWL".into()))?,
            );
            let egl_unbind_wayland_display_wl = std::mem::transmute(
                egl.get_proc_address("eglUnbindWaylandDisplayWL")
                    .ok_or_else(|| FunctionNotExists("eglUnbindWaylandDisplayWL".into()))?,
            );
            Ok(Self {
                egl_create_image_khr,
                gl_eglimage_target_texture2_does,
                egl_bind_wayland_display_wl,
                egl_unbind_wayland_display_wl,
                extensions,
                wayland_map: Default::default(),
            })
        }
    }
}

pub type TextureId = (NativeTexture, u32);

pub unsafe fn import_dma(
    _buffer: &WlMemoryBuffer,
    dma_buffer: &DmaBuffer,
    display: EGLDisplay,
    egl_state: &mut EglState,
) -> Result<(EGLImage, IVec2)> {
    let mut out: Vec<c_int> = Vec::with_capacity(50);
    let planes = dma_buffer.planes.lock().unwrap();

    out.extend([
        khronos_egl::WIDTH,
        dma_buffer.size.x,
        khronos_egl::HEIGHT,
        dma_buffer.size.y,
        LINUX_DRM_FOURCC_EXT as i32,
        dma_buffer.format as i32,
    ]);

    let names = [
        [
            DMA_BUF_PLANE0_FD_EXT,
            DMA_BUF_PLANE0_OFFSET_EXT,
            DMA_BUF_PLANE0_PITCH_EXT,
            DMA_BUF_PLANE0_MODIFIER_LO_EXT,
            DMA_BUF_PLANE0_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE1_FD_EXT,
            DMA_BUF_PLANE1_OFFSET_EXT,
            DMA_BUF_PLANE1_PITCH_EXT,
            DMA_BUF_PLANE1_MODIFIER_LO_EXT,
            DMA_BUF_PLANE1_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE2_FD_EXT,
            DMA_BUF_PLANE2_OFFSET_EXT,
            DMA_BUF_PLANE2_PITCH_EXT,
            DMA_BUF_PLANE2_MODIFIER_LO_EXT,
            DMA_BUF_PLANE2_MODIFIER_HI_EXT,
        ],
        [
            DMA_BUF_PLANE3_FD_EXT,
            DMA_BUF_PLANE3_OFFSET_EXT,
            DMA_BUF_PLANE3_PITCH_EXT,
            DMA_BUF_PLANE3_MODIFIER_LO_EXT,
            DMA_BUF_PLANE3_MODIFIER_HI_EXT,
        ],
    ];
    for (i, plane) in planes.list.iter().enumerate() {
        let fd = &plane.fd;
        let offset = plane.offset;
        let stride = plane.stride;
        out.extend([
            names[i][0] as i32,
            fd.as_raw_fd(),
            names[i][1] as i32,
            offset as i32,
            names[i][2] as i32,
            stride as i32,
        ]);
        if planes.list[0].modifier() != DrmModifier::Invalid
            && planes.list[0].modifier() != DrmModifier::Linear
        {
            out.extend([
                names[i][3] as i32,
                planes.list[0].modifier_lo as i32,
                names[i][4] as i32,
                planes.list[0].modifier_hi as i32,
            ])
        }
    }

    out.push(NONE);

    let image = (egl_state.egl_create_image_khr)(
        display,
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );

    if image == null_mut() {
        Err(FailedToCreateDmaImage.into())
    } else {
        Ok((image, dma_buffer.size))
    }
}
pub unsafe fn image_target_renderbuffer_storage_oes(
    fn_bind_image: extern "system" fn(target: Enum, image: *const c_void),
    raw_image: EGLImage,
) -> Result<()> {
    fn_bind_image(glow::RENDERBUFFER, raw_image);
    Ok(())
}
pub unsafe fn image_target_texture_oes(
    gl: &glow::Context,
    raw_image: EGLImage,
    egl_state: &mut EglState,
) -> Result<NativeTexture> {
    let texture = gl.create_texture().map_err(FailedToCreateSurface)?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    (egl_state.gl_eglimage_target_texture2_does)(glow::TEXTURE_2D, raw_image);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(texture)
}

pub unsafe fn import_memory(
    surface: &WlSurface,
    buffer: &WlMemoryBuffer,
    gl: &glow::Context,
    dest: TextureId,
) -> Result<()> {
    let _offset = buffer.offset;
    let width = buffer.size.x;
    let height = buffer.size.y;
    let stride = buffer.stride;
    let pixelsize = 4i32;
    let shm_inner = buffer.pool.read().unwrap();
    let ptr = std::ptr::from_raw_parts::<[u8]>(
        shm_inner.ptr.as_ptr().offset(buffer.offset as isize).cast(),
        (width * height * 4) as usize,
    )
    .as_ref()
    .unwrap();
    assert!(
        (buffer.offset + (buffer.stride * height) - buffer.stride + width * pixelsize) as usize
            <= shm_inner.size
    );
    gl.bind_texture(dest.1, Some(dest.0));
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_S,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_WRAP_T,
        glow::CLAMP_TO_EDGE as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MIN_FILTER,
        glow::NEAREST as i32,
    );
    gl.tex_parameter_i32(
        glow::TEXTURE_2D,
        glow::TEXTURE_MAG_FILTER,
        glow::NEAREST as i32,
    );

    gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, stride / pixelsize);

    // let image: ImageBuffer<Rgba<u8>, Vec<_>> =
    //     image::ImageBuffer::from_vec(width as u32, height as u32, ptr.to_vec()).unwrap();
    // let snapshtip_count = std::fs::read_dir(".snapshot").unwrap().count();
    // image.save(format!(".snapshot/snapshot_{}.png", snapshtip_count + 1))?;
    // dbg!(ptr.iter().map(|v| *v as usize).sum::<usize>() / ptr.len());

    let (gl_format, _shader_idx) = match buffer.format {
        wl_shm::Format::Abgr8888 => (glow::RGBA, 0),
        wl_shm::Format::Xbgr8888 => (glow::RGBA, 1),
        wl_shm::Format::Argb8888 => (glow::BGRA, 0),
        wl_shm::Format::Xrgb8888 => (glow::BGRA, 1),
        format => return Err(UnsupportedFormat(format).into()),
    };
    if surface.commited.damages.is_empty() {
        trace!(surface=%WlResource::id(&surface.raw),"import {:?}", IRect::new(0, 0, width, height));
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            0,
            0,
            width,
            height,
            gl_format,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(ptr),
        );
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
    } else {
        for region in surface.commited.damages.iter() {
            trace!(surface=%WlResource::id(&surface.raw),"import {:?}", region);
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, region.pos().x.max(0));
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, region.pos().y.max(0));
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                region.pos().x.max(0),
                region.pos().y.max(0),
                region.size().x.min(width),
                region.size().y.min(height),
                gl_format,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(ptr),
            );
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        }
    }
    gl.generate_mipmap(glow::TEXTURE_2D);
    //glTexEnvi(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_REPLACE);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(())
}

pub unsafe fn import_egl(
    buffer: &WlMemoryBuffer,
    egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>,
    gl: &glow::Context,
    display: khronos_egl::Display,
    egl_state: &mut EglState,
    dest: TextureId,
) -> Result<(), ImportSurfaceError> {
    let buffer_guard = buffer.raw.lock().unwrap();
    let egl_surface: khronos_egl::Surface =
        khronos_egl::Surface::from_ptr(buffer_guard.id().as_ptr() as _);

    let width = egl
        .query_surface(display, egl_surface, khronos_egl::WIDTH)
        .map_err(|e| FailedToImportEglBuffer)?;
    let height = egl
        .query_surface(display, egl_surface, khronos_egl::HEIGHT)
        .map_err(|e| FailedToImportEglBuffer)?;
    // let format = egl.query_surface(display, egl_surface, khronos_egl::TEXTURE_FORMAT).map_err(|e|FailedToImportEglBuffer)?;
    // dbg!(width,height,format);
    // let _image_count = match format {
    //     khronos_egl::TEXTURE_RGB => 1,
    //     khronos_egl::TEXTURE_RGBA => 1,
    //     // Format::RGB | Format::RGBA | Format::External => 1,
    //     // Format::Y_UV | Format::Y_XUXV => 2,
    //     // Format::Y_U_V => 3,
    //     _ => panic!(),
    // };
    // let inverted = egl.query_surface(*display, egl_surface, 0x31DB)?;

    let out = [WAYLAND_PLANE_WL as i32, 0_i32, khronos_egl::NONE];
    let image = (egl_state.egl_create_image_khr)(
        display.as_ptr(),
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );
    if image == EGL_NO_IMAGE_KHR {
        return Err(FailedToImportEglBuffer);
    }
    dbg!(image);
    gl.bind_texture(dest.1, Some(dest.0));
    (egl_state.gl_eglimage_target_texture2_does)(glow::TEXTURE_2D, image);
    Ok(())
}

pub unsafe fn import_buffer(
    gl: &glow::Context,
    gl_eglimage_target_renderbuffer_storage_oes: extern "system" fn(
        target: Enum,
        image: *const c_void,
    ),
    egl_image: EGLImage,
) -> Result<NativeRenderbuffer> {
    let render_buffer = gl
        .create_renderbuffer()
        .map_err(FailedToCreateRenderBuffer)?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(render_buffer));
    gl_eglimage_target_renderbuffer_storage_oes(glow::RENDERBUFFER, egl_image);
    Ok(render_buffer)
}

pub unsafe fn create_gpu_image(
    device: &wgpu::Device,
    raw_image: NonZeroU32,
    size: IVec2,
) -> Result<GpuImage> {
    let texture_format = wgpu::TextureFormat::Rgba8Unorm;
    let hal_texture: <Gles as Api>::Texture = device.as_hal::<Gles, _, _>(|hal_device| {
        Result::<_, anyhow::Error>::Ok(hal_device.ok_or_else(|| FailedToGetHal)?.texture_from_raw(
            raw_image,
            &wgpu_hal::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size.x as u32,
                    height: size.y as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: texture_format,
                memory_flags: MemoryFlags::empty(),
                usage: TextureUses::COPY_DST,
                view_formats: vec![texture_format],
            },
            None,
        ))
    })?;
    let wgpu_texture = device.create_texture_from_hal::<Gles>(
        hal_texture,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[texture_format],
        },
    );
    let texture: wgpu::Texture = wgpu_texture;
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(texture_format),
        dimension: None,
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1.try_into().unwrap()),
        base_array_layer: 0,
        array_layer_count: None,
    });
    let sampler: wgpu::Sampler = device.create_sampler(&SamplerDescriptor {
        label: None,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Nearest,
        mipmap_filter: FilterMode::Nearest,
        compare: None,
        anisotropy_clamp: None,
        border_color: None,
        address_mode_u: Default::default(),
        address_mode_v: Default::default(),
        address_mode_w: Default::default(),
        lod_min_clamp: Default::default(),
        lod_max_clamp: Default::default(),
    });
    let image = GpuImage {
        texture: texture.into(),
        texture_view: texture_view.into(),
        texture_format,
        sampler: sampler.into(),
        size: Vec2::new(size.x as f32, size.y as f32),
        mip_level_count: 1,
    };
    Ok(image)
}
pub enum BufferType {
    Shm,
    Egl,
    Dma,
}
pub fn bind_wayland(
    display_handles: &DWayDisplayHandles,
    state: &mut EglState,
    device: &wgpu::Device,
) -> Result<()> {
    unsafe {
        let display: khronos_egl::Display = device.as_hal::<Gles, _, _>(|hal_device| {
            hal_device
                .ok_or_else(|| BackendIsNotEGL)?
                .context()
                .raw_display()
                .cloned()
                .ok_or_else(|| DisplayNotAvailable)
        })?;
        state.bind_wayland(&display_handles.map, display);
    }
    Ok(())
}

pub fn import_wl_surface(
    surface: &WlSurface,
    buffer: &WlMemoryBuffer,
    dma_buffer: Option<&DmaBuffer>,
    egl_buffer: Option<&EGLBuffer>,
    texture: &Texture,
    device: &wgpu::Device,
    egl_state: &mut Option<EglState>,
) -> Result<()> {
    unsafe {
        let display: khronos_egl::Display = device.as_hal::<Gles, _, _>(|hal_device| {
            hal_device
                .ok_or_else(|| BackendIsNotEGL)?
                .context()
                .raw_display()
                .cloned()
                .ok_or_else(|| DisplayNotAvailable)
        })?;
        let mut texture_id = None;
        texture.as_hal::<Gles, _>(|texture| {
            let texture = texture.unwrap();
            // debug!("dest texture: {:?}",texture);
            match &texture.inner {
                wgpu_hal::gles::TextureInner::Texture { raw, target } => {
                    texture_id = Some((*raw, *target));
                }
                _ => {}
            }
        });
        let Some(texture_id) = texture_id else {
            return Err(FailedToGetHal.into());
        };
        device.as_hal::<Gles, _, _>(|hal_device| {
            let hal_device = hal_device.ok_or_else(|| BackendIsNotEGL)?;
            let egl_context = hal_device.context();
            let gl: &glow::Context = &egl_context.lock();
            gl.enable(glow::DEBUG_OUTPUT);
            gl.debug_message_callback(gl_debug_message_callback);
            let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> =
                egl_context.egl_instance().ok_or_else(|| {
                    gl.disable(glow::DEBUG_OUTPUT);
                    BackendIsNotEGL
                })?;
            let egl_state = egl_state.get_or_insert_with(|| EglState::new(gl, egl).unwrap());
            match import_egl(buffer, egl, gl, display, egl_state, texture_id) {
                Err(FailedToImportEglBuffer) => {}
                Ok(()) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
            if let Some(dma_buffer) = dma_buffer {
                let (raw_image, _size) =
                    import_dma(buffer, dma_buffer, display.as_ptr(), egl_state)?;
                let _texture = image_target_texture_oes(gl, raw_image, egl_state)?;
            } else {
                import_memory(surface, buffer, gl, texture_id)?;
            }
            gl.disable(glow::DEBUG_OUTPUT);
            Ok(())
        })
    }
}
pub fn gl_debug_message_callback(source: u32, gltype: u32, id: u32, _severity: u32, message: &str) {
    let source_str = match source {
        glow::DEBUG_SOURCE_API => "API",
        glow::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
        glow::DEBUG_SOURCE_SHADER_COMPILER => "ShaderCompiler",
        glow::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
        glow::DEBUG_SOURCE_APPLICATION => "Application",
        glow::DEBUG_SOURCE_OTHER => "Other",
        _ => unreachable!(),
    };

    let type_str = match gltype {
        glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
        glow::DEBUG_TYPE_ERROR => "Error",
        glow::DEBUG_TYPE_MARKER => "Marker",
        glow::DEBUG_TYPE_OTHER => "Other",
        glow::DEBUG_TYPE_PERFORMANCE => "Performance",
        glow::DEBUG_TYPE_POP_GROUP => "Pop Group",
        glow::DEBUG_TYPE_PORTABILITY => "Portability",
        glow::DEBUG_TYPE_PUSH_GROUP => "Push Group",
        glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
        _ => unreachable!(),
    };

    let _ = std::panic::catch_unwind(|| {
        info!(
            "GLES: [{}/{}] ID {} : {}",
            source_str, type_str, id, message
        );
    });
}
