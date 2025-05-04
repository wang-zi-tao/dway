use std::{os::fd::AsRawFd, ptr::null_mut};

use anyhow::{anyhow, bail, Context, Result};
use bevy::utils::HashSet;
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use gbm::EGLImage;
use glow::HasContext;
use khronos_egl::{Boolean, EGLClientBuffer, EGLContext, EGLDisplay, Enum, Int};
use measure_time::debug_time;
use scopeguard::defer;
use tracing::{debug, trace};
use wgpu_hal::{
    api::Gles,
    gles::{Device, Texture, TextureInner},
};

pub fn call_gl<R>(gl: &glow::Context, f: impl FnOnce() -> R) -> Result<R, TtyRenderError> {
    let r = f();
    let err = unsafe { gl.get_error() };
    if err != 0 {
        return Err(TtyRenderError::GLError(err));
    }
    Ok(r)
}

use super::{TtyRender, TtyRenderError};
use crate::{
    drm::{surface::DrmSurface, DrmDevice},
    gbm::{buffer::GbmBuffer, GbmDevice},
};
pub type EGLInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_4>;

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

pub struct Swapchain {
    render_buffer: glow::Renderbuffer,
    buffer: GbmBuffer,
}

pub struct Surface {
    frame_buffer: glow::Framebuffer,
    render_buffer: glow::Renderbuffer,
}

pub struct GlTtyRender {
    functions: GlesRenderFunctions,
    formats: Vec<DrmFormat>,
}

impl TtyRender for GlTtyRender {
    type Api = Gles;
    type Surface = Surface;
    type Swapchain = Swapchain;

    #[tracing::instrument(skip_all)]
    unsafe fn create_swapchain(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        drm_surface: &DrmSurface,
        drm: &DrmDevice,
        gbm: &GbmDevice,
    ) -> Result<Self::Swapchain, TtyRenderError> {
        let egl_context = device.context();
        let gl = &egl_context.lock();
        let egl_display = egl_context
            .raw_display()
            .ok_or_else(|| anyhow!("egl display is not valid"))?;

        let surface_guard = drm_surface.inner.lock().unwrap();
        let buffer = gbm.create_buffer(
            drm,
            surface_guard.size(),
            surface_guard.formats(),
            &self.formats,
        )?;

        let render_buffer =
            do_create_renderbuffer(gl, &buffer, egl_display.as_ptr(), &self.functions)?;

        debug!("swapchain created");

        Ok(Swapchain {
            render_buffer,
            buffer,
        })
    }

    #[tracing::instrument(skip_all)]
    unsafe fn acquire_surface(
        &mut self,
        device: &Device,
        swapchain: &mut Self::Swapchain,
    ) -> Result<Self::Surface, TtyRenderError> {
        let egl_context = device.context();
        let gl = &egl_context.lock();

        let frame_buffer = gl
            .create_framebuffer()
            .map_err(|m| anyhow!("failed to create framebuffer: {m}"))?;

        debug!("surface created");

        Ok(Surface {
            frame_buffer,
            render_buffer: swapchain.render_buffer,
        })
    }

    unsafe fn discard_surface(
        &mut self,
        device: &Device,
        surface: Self::Surface,
    ) -> Result<(), TtyRenderError> {
        let egl_context = device.context();
        let gl = &egl_context.lock();

        gl.delete_framebuffer(surface.frame_buffer);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    unsafe fn copy_image(
        &mut self,
        device: &Device,
        surface: &mut Self::Surface,
        image: &Texture,
    ) -> Result<(), TtyRenderError> {
        let width = image.copy_size.width as i32;
        let height = image.copy_size.height as i32;
        let egl_context = device.context();
        let gl = &egl_context.lock();

        let TextureInner::Texture {
            raw: src_raw,
            target: src_target,
        } = &image.inner
        else {
            return Err(TtyRenderError::TextureIsNotValid);
        };


        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(surface.frame_buffer));
        defer! {gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, None);};

        call_gl(gl, || {
            gl.framebuffer_renderbuffer(
                glow::DRAW_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::RENDERBUFFER,
                Some(surface.render_buffer),
            )
        })?;

        let texture_framebuffer = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(texture_framebuffer));
        defer! {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
            gl.delete_framebuffer(texture_framebuffer);
        };
        gl.bind_texture(*src_target, Some(*src_raw));
        gl.framebuffer_texture_2d(
            glow::READ_FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(*src_raw),
            0,
        );

        gl.disable(glow::FRAMEBUFFER_SRGB);
        call_gl(gl, || {
            gl.blit_framebuffer(0, 0, width, height, 0, 0, width, height, glow::COLOR_BUFFER_BIT, glow::LINEAR)
        })?;
        gl.enable(glow::FRAMEBUFFER_SRGB);

        {
            debug_time!("wait gl operation");
            gl.flush();
            gl.finish();
        }

        debug!("copy image to surface");

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    fn new(device: &<Self::Api as wgpu_hal::Api>::Device) -> Result<Self, TtyRenderError>
    where
        Self: Sized,
    {
        let egl_context = device.context();
        let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
            .egl_instance()
            .ok_or_else(|| TtyRenderError::BackendIsNotEGL)?;

        let functions = GlesRenderFunctions::new(egl)?;
        let formats = get_formats(&functions, device)?;

        Ok(Self { functions, formats })
    }

    unsafe fn commit(
        &mut self,
        swapchain: &mut Self::Swapchain,
        _surface: &mut Self::Surface,
        drm_surface: &DrmSurface,
        drm: &DrmDevice,
    ) -> Result<()> {
        drm_surface.commit_buffer(drm, &swapchain.buffer)
    }
}

pub fn call_egl_boolean(
    egl: &EGLInstance,
    f: impl FnOnce() -> Boolean,
) -> Result<(), TtyRenderError> {
    let r = f();
    if r != khronos_egl::TRUE {
        if let Some(err) = egl.get_error() {
            Err(TtyRenderError::EglError(err))
        } else {
            Err(TtyRenderError::UnknownEglError)
        }
    } else {
        Ok(())
    }
}

pub fn call_egl_vec<T: Default>(
    egl: &EGLInstance,
    mut f: impl FnMut(Int, *mut T, *mut Int) -> Boolean,
) -> Result<Vec<T>, TtyRenderError> {
    let mut num = 0;
    call_egl_boolean(egl, || f(0, null_mut(), &mut num))?;
    if num == 0 {
        return Ok(vec![]);
    }
    let mut vec = Vec::new();
    vec.resize_with(num as usize, || Default::default());
    call_egl_boolean(egl, || f(num, vec.as_mut_ptr() as *mut T, &mut num))?;
    Ok(vec)
}

pub fn call_egl_double_vec<T1: Default, T2: Default>(
    egl: &EGLInstance,
    mut f: impl FnMut(Int, *mut T1, *mut T2, *mut Int) -> Boolean,
) -> Result<(Vec<T1>, Vec<T2>), khronos_egl::Error> {
    let on_error = |egl: &EGLInstance| {
        if let Some(err) = egl.get_error() {
            if err == khronos_egl::Error::BadParameter {
                Ok((vec![], vec![]))
            } else {
                Err(err)
            }
        } else {
            Ok((vec![], vec![]))
        }
    };
    let mut num = 0;
    if f(0, null_mut(), null_mut(), &mut num) != khronos_egl::TRUE {
        return on_error(egl);
    }
    if num == 0 {
        return Ok((vec![], vec![]));
    }
    let mut vec1 = Vec::new();
    vec1.resize_with(num as usize, || Default::default());
    let mut vec2 = Vec::new();
    vec2.resize_with(num as usize, || Default::default());
    if f(
        num,
        vec1.as_mut_ptr() as *mut T1,
        vec2.as_mut_ptr() as *mut T2,
        &mut num,
    ) != khronos_egl::TRUE
    {
        return on_error(egl);
    }
    Ok((vec1, vec2))
}

pub fn get_egl_extensions(
    egl: &EGLInstance,
    egl_display: khronos_egl::Display,
) -> Result<HashSet<String>> {
    Ok(egl
        .query_string(Some(egl_display), khronos_egl::EXTENSIONS)?
        .to_string_lossy()
        .split(' ')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string())
        .collect())
}

pub struct GlesRenderFunctions {
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
impl GlesRenderFunctions {
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

pub fn get_formats(
    functions: &GlesRenderFunctions,
    hal_device: &<Gles as wgpu_hal::Api>::Device,
) -> Result<Vec<DrmFormat>> {
    let egl_context = hal_device.context();
    let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
        .egl_instance()
        .ok_or_else(|| TtyRenderError::BackendIsNotEGL)?;
    let egl_display = egl_context
        .raw_display()
        .ok_or_else(|| TtyRenderError::EglInstanceIsNotInitialized)?;

    let extensions = get_egl_extensions(egl, *egl_display)?;
    let fourcc_list = if !extensions.contains("EGL_EXT_image_dma_buf_import_modifiers") {
        vec![DrmFourcc::Argb8888, DrmFourcc::Xrgb8888]
    } else {
        call_egl_vec(egl, |num, vec, p_num| {
            (functions.egl_query_dmabuf_format_ext)(egl_display.as_ptr(), num, vec, p_num)
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
        .map_err(TtyRenderError::EglError)?;
        if mods.is_empty() {
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

    Ok(render_formats.into_iter().collect())
}

unsafe fn do_create_renderbuffer(
    gl: &glow::Context,
    buffer: &GbmBuffer,
    display: EGLDisplay,
    functions: &GlesRenderFunctions,
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
    trace!("eglCreateImageKHR({request:?})");

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
    call_gl(gl, || {
        (functions.gl_eglimage_target_renderbuffer_storage_oes)(glow::RENDERBUFFER, image)
    })
    .with_context(|| "EGLImageTargetRenderbufferStorageOES")?;
    gl.bind_renderbuffer(glow::RENDERBUFFER, None);

    Ok(renderbuffer)
}
