use std::os::fd::AsRawFd;
use std::ptr::null_mut;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use bevy::utils::HashSet;
use drm_fourcc::DrmFormat;
use drm_fourcc::DrmFourcc;
use drm_fourcc::DrmModifier;
use gbm::EGLImage;
use glow::HasContext;
use khronos_egl::EGLClientBuffer;
use khronos_egl::EGLContext;
use khronos_egl::EGLDisplay;
use khronos_egl::Enum;
use khronos_egl::{Boolean, Int};
use tracing::debug;
use wgpu::Extent3d;
use wgpu::TextureDimension;
use wgpu::TextureFormat;
use wgpu_hal::gles::Device;
use wgpu_hal::gles::Texture;

use crate::gbm::buffer::GbmBuffer;

use super::RenderCache;
use super::TtyRenderState;
use wgpu_hal::{api::Gles, MemoryFlags, TextureUses};

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

pub fn call_egl_boolean(egl: &EGLInstance, f: impl FnOnce() -> Boolean) -> Result<()> {
    let r = f();
    if r != khronos_egl::TRUE {
        if let Some(err) = egl.get_error() {
            Err(anyhow!("egl error: {:?}", err))
        } else {
            Err(anyhow!("unknown egl error"))
        }
    } else {
        Ok(())
    }
}

pub fn call_egl_vec<T: Default>(
    egl: &EGLInstance,
    mut f: impl FnMut(Int, *mut T, *mut Int) -> Boolean,
) -> Result<Vec<T>> {
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
                return Ok((vec![], vec![]));
            } else {
                return Err(err);
            }
        } else {
            return Ok((vec![], vec![]));
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

pub struct GlesRenderCache {
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
impl GlesRenderCache {
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
    cache: &mut RenderCache,
    render_device: &wgpu::Device,
) -> Option<Result<Vec<DrmFormat>>> {
    unsafe {
        render_device.as_hal::<Gles, _, _>(|hal_device| {
            hal_device.map(|hal_device| {
                let egl_context = hal_device.context();
                let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
                    .egl_instance()
                    .ok_or_else(|| anyhow!("gpu backend is not egl"))?;
                let egl_display = egl_context
                    .raw_display()
                    .ok_or_else(|| anyhow!("egl display is not valid"))?;

                let functions = match cache {
                    RenderCache::None => {
                        let functions = GlesRenderCache::new(egl)?;
                        *cache = RenderCache::Gles(functions);
                        let RenderCache::Gles(functions) = &cache else {
                            unreachable!();
                        };
                        functions
                    }
                    RenderCache::Gles(g) => g,
                };

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

                Result::<_, anyhow::Error>::Ok(render_formats.into_iter().collect())
            })
        })
    }
}

pub fn create_framebuffer_texture(
    state: &mut TtyRenderState,
    hal_device: &Device,
    buffer: &GbmBuffer,
) -> Result<Texture> {
    unsafe {
        let egl_context = hal_device.context();
        let gl: &glow::Context = &egl_context.lock();
        let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> = egl_context
            .egl_instance()
            .ok_or_else(|| anyhow!("gpu backend is not egl"))?;
        let egl_display = egl_context
            .raw_display()
            .ok_or_else(|| anyhow!("egl display is not valid"))?;

        let functions = match &state.cache {
            RenderCache::None => {
                let functions = GlesRenderCache::new(egl)?;
                state.cache = RenderCache::Gles(functions);
                let RenderCache::Gles(functions) = &state.cache else {
                    unreachable!();
                };
                functions
            }
            RenderCache::Gles(g) => g,
        };

        let renderbuffer = do_create_renderbuffer(gl, buffer, egl_display.as_ptr(), functions)?;

        let hal_texture = hal_device.texture_from_raw_renderbuffer(
            renderbuffer.0,
            &wgpu_hal::TextureDescriptor {
                label: Some("gbm renderbuffer"),
                size: Extent3d {
                    width: buffer.size.x as u32,
                    height: buffer.size.y as u32,
                    depth_or_array_layers: 1,
                    ..Default::default()
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
        Ok(hal_texture)
    }
}

unsafe fn do_create_renderbuffer(
    gl: &glow::Context,
    buffer: &GbmBuffer,
    display: EGLDisplay,
    functions: &GlesRenderCache,
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
    Ok(renderbuffer)
}
