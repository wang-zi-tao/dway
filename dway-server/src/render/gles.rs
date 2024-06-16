use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, c_int, c_void},
    num::NonZeroU32,
    os::fd::AsRawFd,
    ptr::null_mut,
};

use bevy::{ecs::entity::EntityHashMap, prelude::info, render::texture::GpuImage};
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use dway_util::formats::ImageFormat;
use glow::{HasContext, NativeRenderbuffer, NativeTexture, PixelPackData};
use image::{ImageBuffer, Rgba};
use khronos_egl::{
    Attrib, Boolean, EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, Enum, Int, NONE,
};
use scopeguard::defer;
use wayland_backend::server::WeakHandle;
use wgpu::{FilterMode, SamplerDescriptor, Texture, TextureAspect};
use wgpu_hal::{api::Gles, Api, MemoryFlags, TextureUses};
use DWayRenderError::*;

use super::{
    drm::{DrmInfo, DrmNode},
    importnode::DWayDisplayHandles,
    util::*,
};
use crate::{
    prelude::*,
    util::rect::IRect,
    wl::{
        buffer::{UninitedWlBuffer, WlShmBuffer},
        surface::WlSurface,
    },
    zwp::dmabufparam::DmaBuffer,
};

#[derive(Debug)]
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
    pub wayland_map: EntityHashMap<WeakHandle>,
}
impl EglState {
    pub fn bind_wayland(
        &mut self,
        wayland_map: &EntityHashMap<DisplayHandle>,
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

pub fn drm_info(device: &wgpu::Device) -> Result<DrmInfo, DWayRenderError> {
    with_gl(device, |context, egl, _gl| {
        info!("use gl");
        egl_check_extensions(egl, &["EGL_EXT_device_base", "EGL_EXT_device_query"])?;
        let egl_display = context.raw_display().ok_or_else(|| DisplayNotAvailable)?;

        let query_display_attrib_ext: extern "system" fn(EGLDisplay, Int, *mut Attrib) -> Boolean =
            unsafe { std::mem::transmute(get_egl_function(egl, "eglQueryDisplayAttribEXT")?) };
        let query_dmabuf_format_ext: extern "system" fn(
            EGLDisplay,
            Int,
            *mut u32,
            *mut Int,
        ) -> Boolean =
            unsafe { std::mem::transmute(get_egl_function(egl, "eglQueryDmaBufFormatsEXT")?) };
        let query_device_string_ext: extern "system" fn(Attrib, Int) -> *const c_char =
            unsafe { std::mem::transmute(get_egl_function(egl, "eglQueryDeviceStringEXT")?) };
        let query_dma_buf_modifiers_ext: extern "system" fn(
            EGLDisplay,
            Int,
            Int,
            *mut u64,
            *mut Boolean,
            *mut Int,
        ) -> Boolean =
            unsafe { std::mem::transmute(get_egl_function(egl, "eglQueryDmaBufModifiersEXT")?) };

        let extensions = get_egl_extensions(egl)?;

        let mut device: Attrib = 0;
        call_egl_boolean(egl, || {
            query_display_attrib_ext(egl_display.as_ptr(), DEVICE_EXT, &mut device)
        })?;
        if device == NO_DEVICE_EXT {
            return Err(EglApiError("eglQueryDisplayAttribEXT"));
        }
        let device_extensions = get_extensions(|| {
            call_egl_string(egl, || {
                query_device_string_ext(device, khronos_egl::EXTENSIONS)
            })
            .map(|s| s.to_string_lossy().to_string())
        })
        .map_err(|e| Unknown(anyhow!("failed to get device extensions: {e}")))?;
        check_extensions(
            &device_extensions,
            &["EGL_EXT_device_drm_render_node", "EGL_EXT_device_drm"],
        )?;

        let path = call_egl_string(egl, || {
            query_device_string_ext(device, DRM_RENDER_NODE_FILE_EXT)
        })
        .or_else(|_| call_egl_string(egl, || query_device_string_ext(device, DRM_DEVICE_FILE_EXT)))
        .map_err(|e| Unknown(anyhow!("failed to get device path: {e}")))?;
        let drm_node = DrmNode::new(path)?;

        let formats = if !extensions.contains("EGL_EXT_image_dma_buf_import_modifiers") {
            vec![DrmFourcc::Argb8888, DrmFourcc::Xrgb8888]
        } else {
            call_egl_vec(egl, |num, vec, p_num| {
                query_dmabuf_format_ext(egl_display.as_ptr(), num, vec, p_num)
            })?
            .into_iter()
            .map(|f| f.try_into())
            .try_collect()
            .map_err(|e| Unknown(anyhow!("unknown format: {e}")))?
        };

        let mut texture_formats = HashSet::new();
        let mut render_formats = HashSet::new();
        for fourcc in formats.iter().cloned() {
            let (mods, external) = call_egl_double_vec(egl, |num, vec1, vec2, p_num| {
                query_dma_buf_modifiers_ext(
                    egl_display.as_ptr(),
                    fourcc as i32,
                    num,
                    vec1,
                    vec2,
                    p_num,
                )
            })?;
            if mods.is_empty() {
                texture_formats.insert(DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::Invalid,
                });
                render_formats.insert(DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::Invalid,
                });
            }
            for (modifier, external_only) in mods.into_iter().zip(external.into_iter()) {
                let format = DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::from(modifier),
                };
                texture_formats.insert(format);
                if external_only == 0 {
                    render_formats.insert(format);
                }
            }
        }

        Ok(DrmInfo {
            texture_formats: texture_formats.into_iter().collect(),
            render_formats: render_formats.into_iter().collect(),
            drm_node,
        })
    })
}

pub type TextureId = (NativeTexture, u32);

#[tracing::instrument(skip_all)]
pub unsafe fn import_dma(
    gl: &glow::Context,
    dma_buffer: &DmaBuffer,
    display: EGLDisplay,
    egl_state: &mut EglState,
    dest: TextureId,
) -> Result<()> {
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
        return Err(FailedToCreateDmaImage.into());
    }

    image_bind_texture(gl, image, egl_state, dest, dma_buffer.size)?;
    Ok(())
}

#[tracing::instrument(skip_all)]
pub unsafe fn image_bind_texture(
    gl: &glow::Context,
    raw_image: EGLImage,
    egl_state: &mut EglState,
    dest: TextureId,
    size: IVec2,
) -> Result<()> {
    let texture = gl
        .create_texture()
        .map_err(|s| anyhow!("failed to create texture: {s}"))?;
    defer! {
        gl.delete_texture(texture);
    }

    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    call_gl(gl, || {
        (egl_state.gl_eglimage_target_texture2_does)(glow::TEXTURE_2D, raw_image);
    })?;
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.copy_image_sub_data(
        texture,
        glow::TEXTURE_2D,
        0,
        0,
        0,
        0,
        dest.0,
        glow::TEXTURE_2D,
        0,
        0,
        0,
        0,
        size.x,
        size.y,
        1,
    );
    Ok(())
}

#[tracing::instrument(skip_all)]
pub unsafe fn import_shm(
    surface: &WlSurface,
    buffer: &WlShmBuffer,
    gl: &glow::Context,
    dest: TextureId,
) -> Result<()> {
    let _offset = buffer.offset;
    let width = buffer.size.x;
    let height = buffer.size.y;
    let stride = buffer.stride;
    let pixelsize = 4i32;
    let shm_inner = buffer.pool.read().unwrap();
    let slice = shm_inner.as_slice(buffer)?;
    assert!(
        (buffer.offset + (buffer.stride * height) - buffer.stride + width * pixelsize) as usize
            <= shm_inner.size
    );
    gl.bind_texture(glow::TEXTURE_2D, Some(dest.0));
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

    let gl_format = ImageFormat::from_wayland_format(buffer.format)?.gles_format;
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
            glow::PixelUnpackData::Slice(slice),
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
                glow::PixelUnpackData::Slice(slice),
            );
            gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
            gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        }
    }
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(())
}

#[tracing::instrument(skip_all)]
pub unsafe fn import_egl(
    buffer: &wl_buffer::WlBuffer,
    egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4>,
    gl: &glow::Context,
    display: khronos_egl::Display,
    egl_state: &mut EglState,
    dest: TextureId,
) -> Result<(), DWayRenderError> {
    let egl_surface: khronos_egl::Surface =
        khronos_egl::Surface::from_ptr(buffer.id().as_ptr() as _);

    let width = egl
        .query_surface(display, egl_surface, khronos_egl::WIDTH)
        .map_err(|_| FailedToImportEglBuffer)?;
    let height = egl
        .query_surface(display, egl_surface, khronos_egl::HEIGHT)
        .map_err(|_| FailedToImportEglBuffer)?;

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
    image_bind_texture(gl, image, egl_state, dest, IVec2::new(width, height))?;
    output_texture("eglbuffer", gl, dest.0, IVec2::new(width, height));
    warn!("import egl buffer");
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
    let hal_texture: <Gles as Api>::Texture = device
        .as_hal::<Gles, _, _>(|hal_device| {
            Result::<_, anyhow::Error>::Ok(
                hal_device.ok_or_else(|| FailedToGetHal)?.texture_from_raw(
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
                ),
            )
        })
        .ok_or(BackendIsIsInvalid)??;
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
        anisotropy_clamp: 1,
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
#[tracing::instrument(skip_all)]
pub fn bind_wayland(
    display_handles: &DWayDisplayHandles,
    state: &mut EglState,
    device: &wgpu::Device,
) -> Result<()> {
    let display = get_egl_display(device)?;
    state.bind_wayland(&display_handles.map, display);
    Ok(())
}

#[tracing::instrument(skip_all)]
pub fn import_wl_surface(
    surface: &WlSurface,
    shm_buffer: Option<&WlShmBuffer>,
    dma_buffer: Option<&DmaBuffer>,
    egl_buffer: Option<&UninitedWlBuffer>,
    texture: &Texture,
    device: &wgpu::Device,
    egl_state: &mut EglState,
) -> Result<(), DWayRenderError> {
    unsafe {
        let display: khronos_egl::Display = device
            .as_hal::<Gles, _, _>(|hal_device| {
                hal_device
                    .ok_or_else(|| BackendIsNotEGL)?
                    .context()
                    .raw_display()
                    .cloned()
                    .ok_or_else(|| DisplayNotAvailable)
            })
            .ok_or(BackendIsIsInvalid)??;
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
            return Err(FailedToGetHal);
        };
        device
            .as_hal::<Gles, _, _>(|hal_device| {
                let hal_device = hal_device.ok_or_else(|| BackendIsNotEGL)?;
                let egl_context = hal_device.context();
                let gl: &glow::Context = &egl_context.lock();
                let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> =
                    egl_context.egl_instance().ok_or_else(|| {
                        gl.disable(glow::DEBUG_OUTPUT);
                        BackendIsNotEGL
                    })?;
                if let Some(egl_buffer) = egl_buffer {
                    import_egl(&egl_buffer.raw, egl, gl, display, egl_state, texture_id)?;
                } else if let Some(dma_buffer) = dma_buffer {
                    import_dma(gl, dma_buffer, display.as_ptr(), egl_state, texture_id)?;
                } else if let Some(shm_buffer) = shm_buffer {
                    import_shm(surface, shm_buffer, gl, texture_id)?;
                }
                Result::<(), DWayRenderError>::Ok(())
            })
            .ok_or(BackendIsIsInvalid)??;
        Ok(())
    }
}

#[tracing::instrument(skip_all)]
pub fn output_texture(name: &str, gl: &glow::Context, texture: NativeTexture, size: IVec2) {
    unsafe {
        let framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(texture),
            0,
        );
        let mut buffer = vec![0u8; 4 * size.x as usize * size.y as usize];
        gl.read_pixels(
            0,
            0,
            size.x,
            size.y,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelPackData::Slice(&mut buffer[..]),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        dbg!(buffer.iter().map(|v| *v as usize).sum::<usize>() / buffer.len());
        let image: ImageBuffer<Rgba<u8>, Vec<_>> =
            ImageBuffer::from_vec(size.x as u32, size.y as u32, buffer).unwrap();
        let snapshtip_count = std::fs::read_dir(".snapshot").unwrap().count();
        let path = format!(".snapshot/{name}_{}.png", snapshtip_count + 1);
        info!("take snapshot, save at ${path}");
        image.save(&path).unwrap();
    }
}
