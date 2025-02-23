use std::{
    collections::HashSet,
    ffi::{c_char, c_int, c_void},
    mem::take,
    os::fd::AsRawFd,
    sync::Arc,
};

use bevy::{
    ecs::entity::EntityHashMap,
    prelude::info,
    render::{renderer::RenderDevice, texture::GpuImage},
};
use crossbeam_queue::SegQueue;
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use dway_util::formats::ImageFormat;
use glow::{HasContext, NativeRenderbuffer, NativeTexture};
use khronos_egl::{
    Attrib, Boolean, EGLClientBuffer, EGLContext, EGLDisplay, EGLImage, Enum, Int, NONE,
};
use wayland_backend::server::WeakHandle;
use wgpu::Texture;
use wgpu_hal::{api::Gles, DropCallback};
use DWayRenderError::*;

use super::{
    drm::{DrmInfo, DrmNode},
    importnode::{
        drm_fourcc_to_wgpu_format, hal_texture_descriptor, hal_texture_to_gpuimage,
        DWayDisplayHandles,
    },
    util::*,
    ImportDmaBufferRequest,
};
use crate::{
    prelude::*,
    util::rect::IRect,
    wl::{buffer::WlShmBuffer, surface::WlSurface},
};

pub struct DestroyBuffer {
    egl_image: EGLImage,
    texture: NativeTexture,
}
unsafe impl Send for DestroyBuffer {
}
unsafe impl Sync for DestroyBuffer {
}

#[derive(Debug)]
pub struct EglState {
    pub egl_create_image_khr: unsafe extern "system" fn(
        EGLDisplay,
        EGLContext,
        Enum,
        EGLClientBuffer,
        *const Int,
    ) -> EGLImage,
    pub gl_eglimage_target_texture2_does: unsafe extern "system" fn(target: Enum, image: EGLImage),
    pub egl_bind_wayland_display_wl: unsafe extern "system" fn(EGLDisplay, *mut c_void) -> Boolean,
    pub egl_unbind_wayland_display_wl: unsafe extern "system" fn(EGLDisplay, EGLImage) -> Boolean,
    pub extensions: HashSet<String>,
    pub wayland_map: EntityHashMap<WeakHandle>,
    pub destroy_queue: Arc<SegQueue<DestroyBuffer>>,
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
                destroy_queue: Arc::new(Default::default()),
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

pub struct ImageGuard {
    pub egl_image: EGLImage,
    pub texture: NativeTexture,
    pub destroy_queue: Arc<SegQueue<DestroyBuffer>>,
}
unsafe impl Send for ImageGuard {
}
unsafe impl Sync for ImageGuard {
}

impl ImageGuard {
    fn drop_callback(self) -> DropCallback {
        Box::new(move || {
            let _ = self;
        })
    }
}

impl Drop for ImageGuard {
    fn drop(&mut self) {
        self.destroy_queue.push(DestroyBuffer {
            egl_image: self.egl_image,
            texture: self.texture,
        });
    }
}

#[tracing::instrument(skip_all)]
pub unsafe fn create_gles_dma_image(
    gl: &glow::Context,
    display: EGLDisplay,
    egl_state: &EglState,
    buffer_info: &mut ImportDmaBufferRequest,
) -> Result<ImageGuard> {
    let mut out: Vec<c_int> = Vec::with_capacity(50);
    let planes = take(&mut buffer_info.planes);

    out.extend([
        khronos_egl::WIDTH,
        buffer_info.size.x as i32,
        khronos_egl::HEIGHT,
        buffer_info.size.y as i32,
        LINUX_DRM_FOURCC_EXT as i32,
        buffer_info.format as i32,
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
    for (i, plane) in planes.iter().enumerate() {
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
        let modifier = planes[i].modifier;
        if modifier != DrmModifier::Invalid && modifier != DrmModifier::Linear {
            out.extend([
                names[i][3] as i32,
                u64::from(modifier) as i32,
                names[i][4] as i32,
                (u64::from(modifier) >> 32) as i32,
            ])
        }
    }

    out.push(NONE);

    let egl_image = (egl_state.egl_create_image_khr)(
        display,
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );
    if egl_image.is_null() {
        return Err(FailedToCreateDmaImage.into());
    }

    let texture = image_bind_texture(gl, egl_image, egl_state)?;

    Ok(ImageGuard {
        egl_image,
        texture,
        destroy_queue: egl_state.destroy_queue.clone(),
    })
}

#[tracing::instrument(skip_all)]
pub unsafe fn image_bind_texture(
    gl: &glow::Context,
    egl_image: EGLImage,
    egl_state: &EglState,
) -> Result<NativeTexture> {
    let texture = gl
        .create_texture()
        .map_err(|s| anyhow!("failed to create texture: {s}"))?;
    gl.bind_texture(glow::TEXTURE_2D, Some(texture));
    call_gl(gl, || {
        (egl_state.gl_eglimage_target_texture2_does)(glow::TEXTURE_2D, egl_image);
    })?;
    gl.generate_mipmap(glow::TEXTURE_2D);
    gl.bind_texture(glow::TEXTURE_2D, None);
    Ok(texture)
}

pub fn create_wgpu_dma_image(
    device: &wgpu::Device,
    request: &mut ImportDmaBufferRequest,
    egl_state: &EglState,
) -> Result<GpuImage, DWayRenderError> {
    unsafe {
        let format = drm_fourcc_to_wgpu_format(request)?;
        let hal_texture = device
            .as_hal::<Gles, _, _>(|hal_device| {
                let hal_device = hal_device.ok_or_else(|| BackendIsNotEGL)?;
                let egl_context = hal_device.context();
                let gl: &glow::Context = &egl_context.lock();
                let display = egl_context
                    .raw_display()
                    .ok_or_else(|| DisplayNotAvailable)?;
                debug!(size=?request.size, ?format, "create dma image");
                let image_guard = create_gles_dma_image(gl, display.as_ptr(), egl_state, request)?;
                let texture = image_guard.texture;
                let hal_texture = hal_device.texture_from_raw(
                    texture.0,
                    &hal_texture_descriptor(request.size, format)?,
                    Some(image_guard.drop_callback()),
                );
                Result::<_, DWayRenderError>::Ok(hal_texture)
            })
            .ok_or(DWayRenderError::BackendIsIsInvalid)??;
        let gpu_image = hal_texture_to_gpuimage::<Gles>(device, request.size, format, hal_texture)?;
        Ok(gpu_image)
    }
}

#[tracing::instrument(skip_all)]
pub unsafe fn import_raw_shm_buffer(
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
    egl_state: &EglState,
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
    let egl_image = (egl_state.egl_create_image_khr)(
        display.as_ptr(),
        khronos_egl::NO_CONTEXT,
        LINUX_DMA_BUF_EXT,
        std::ptr::null_mut(),
        out.as_ptr(),
    );
    if egl_image == EGL_NO_IMAGE_KHR {
        return Err(FailedToImportEglBuffer);
    }
    let texture = image_bind_texture(gl, egl_image, egl_state)?;
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

pub fn import_shm(
    surface: &WlSurface,
    shm_buffer: &WlShmBuffer,
    texture: &Texture,
    device: &wgpu::Device,
) -> Result<(), DWayRenderError> {
    unsafe {
        device
            .as_hal::<Gles, _, _>(|hal_device| {
                let hal_device = hal_device.ok_or_else(|| BackendIsNotEGL)?;
                let egl_context = hal_device.context();
                let gl: &glow::Context = &egl_context.lock();
                texture.as_hal::<Gles, _, _>(|texture| {
                    let texture = texture.unwrap();
                    if let wgpu_hal::gles::TextureInner::Texture { raw, target } = &texture.inner {
                        import_raw_shm_buffer(surface, shm_buffer, gl, (*raw, *target))?;
                    }
                    Ok(())
                })
            })
            .ok_or(BackendIsIsInvalid)?
    }
}

pub fn clean(state: &EglState, render_device: &RenderDevice) {
    if state.destroy_queue.len() > 0 {
        unsafe {
            render_device
                .wgpu_device()
                .as_hal::<Gles, _, _>(|hal_device| {
                    let Some(hal_device) = hal_device else { return };
                    let egl_context = hal_device.context();
                    let gl: &glow::Context = &egl_context.lock();
                    let Some(display) = egl_context.raw_display() else {
                        return;
                    };
                    while let Some(DestroyBuffer { egl_image, texture }) = state.destroy_queue.pop()
                    {
                        gl.delete_texture(texture);
                        (state.egl_unbind_wayland_display_wl)(display.as_ptr(), egl_image);
                    }
                });
        }
    }
}
