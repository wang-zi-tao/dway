use std::{
    collections::HashSet,
    ffi::{c_char, c_uint, c_void, CStr},
    ptr::null_mut,
};

use crate::prelude::*;
use ash::vk;
use drm_fourcc::DrmFourcc;
use glow::HasContext;
use khronos_egl::{Attrib, Boolean, Int};
use scopeguard::defer;
use thiserror::Error;
use wgpu_hal::{gles::AdapterContext, api::Gles};

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

pub const DRM_RENDER_NODE_FILE_EXT: i32 = 0x3377;
pub const DRM_DEVICE_FILE_EXT: i32 = 0x3233;

pub const EGL_NO_IMAGE_KHR: *mut c_void = null_mut();

pub const DEVICE_EXT: i32 = 0x322C;
pub const NO_DEVICE_EXT: Attrib = 0;

pub const EGL_DEBUG_MSG_CRITICAL_KHR: Attrib = 0x33B9;
pub const EGL_DEBUG_MSG_ERROR_KHR: Attrib = 0x33BA;
pub const EGL_DEBUG_MSG_INFO_KHR: Attrib = 0x33BC;
pub const EGL_DEBUG_MSG_WARN_KHR: Attrib = 0x33BB;

pub const TEXTURE_EXTERNAL_OES: u32 = 0x8D65;

pub type EGLInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_4>;

#[derive(Error, Debug)]
pub enum DWayRenderError {
    #[error("The following EGL extensions is not supported: {0:?}")]
    EglExtensionNotSupported(Vec<String>),
    #[error("unknown error when calling {0}")]
    EglApiError(&'static str),
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
    #[error("gpu backend is not vulkan")]
    BackendIsNotVulkan,
    #[error("no valid memory type index")]
    UnknownBackend,
    #[error("unknown wgpu backend")]
    NoValidMemoryType,
    #[error("failed to create dma image")]
    FailedToCreateDmaImage,
    #[error("failed to create texture: {0}")]
    FailedToCreateSurface(String),
    #[error("failed to create render buffer: {0}")]
    FailedToCreateRenderBuffer(String),
    #[error("imvalud dma buffer")]
    InvalidDmaBuffer,
    #[error("no drm node")]
    NotDrmNode,
    #[error("unsupported format: {0:?}")]
    UnsupportedFormat(wl_shm::Format),
    #[error("unsupported format: {0:?}")]
    UnsupportedDrmFormat(DrmFourcc),
    #[error("gl error: {0:?}")]
    GLError(u32),
    #[error("egl error: {0:?}")]
    EglError(#[from] khronos_egl::Error),
    #[error("vulkan error: {0:?}")]
    VKError(#[from] vk::Result),
    #[error("{0}")]
    Unknown(#[from] anyhow::Error),
    #[error("unknown egl error")]
    UnknownEglError,
}

pub fn get_egl_display(device: &wgpu::Device) -> Result<khronos_egl::Display> {
    unsafe {
        let display: khronos_egl::Display = device.as_hal::<Gles, _, _>(|hal_device| {
            hal_device
                .ok_or_else(|| DWayRenderError::BackendIsNotEGL)?
                .context()
                .raw_display()
                .cloned()
                .ok_or_else(|| DWayRenderError::DisplayNotAvailable)
        })?;
        Ok(display)
    }
}

pub fn check_extensions(supported_extensions: &HashSet<String>, extensions: &[&str]) -> Result<()> {
    let mut unsupported_extensions = vec![];
    for extension in extensions {
        if !supported_extensions.contains(*extension) {
            unsupported_extensions.push(extension.to_string());
        }
    }
    if unsupported_extensions.len() > 0 {
        bail!(DWayRenderError::EglExtensionNotSupported(
            unsupported_extensions
        ));
    }
    Ok(())
}

pub fn gl_check_extensions(gl: &glow::Context, extensions: &[&str]) -> Result<()> {
    let supported_extensions = gl.supported_extensions();
    check_extensions(&supported_extensions, extensions)
}

pub fn egl_check_extensions(egl: &EGLInstance, extensions: &[&str]) -> Result<()> {
    let supported_extensions = get_egl_extensions(egl)?;
    check_extensions(&supported_extensions, extensions)
}

pub fn get_egl_function(egl: &EGLInstance, func: &str) -> Result<extern "C" fn()> {
    Ok(egl
        .get_proc_address(func)
        .ok_or_else(|| DWayRenderError::FunctionNotExists(func.to_string()))?)
}

pub fn with_gl<R>(
    device: &wgpu::Device,
    f: impl FnOnce(&AdapterContext, &EGLInstance, &glow::Context) -> Result<R, DWayRenderError>,
) -> Result<R, DWayRenderError> {
    unsafe {
        device.as_hal::<Gles, _, _>(|hal_device| {
            let context = hal_device
                .ok_or_else(|| DWayRenderError::BackendIsNotEGL)?
                .context();
            let gl: &glow::Context = &context.lock();
            let egl: &EGLInstance = context
                .egl_instance()
                .ok_or_else(|| DWayRenderError::BackendIsNotEGL)?;
            gl.enable(glow::DEBUG_OUTPUT);
            gl.debug_message_callback(gl_debug_message_callback);
            defer! {
                gl.disable(glow::DEBUG_OUTPUT);
            };
            f(context, egl, gl)
        })
    }
}

pub fn call_egl_boolean(
    egl: &EGLInstance,
    f: impl FnOnce() -> Boolean,
) -> Result<(), DWayRenderError> {
    let r = f();
    if r != khronos_egl::TRUE {
        if let Some(err) = egl.get_error() {
            Err(DWayRenderError::EglError(err))
        } else {
            Err(DWayRenderError::UnknownEglError)
        }
    } else {
        Ok(())
    }
}

pub fn call_egl_string(
    egl: &EGLInstance,
    f: impl FnOnce() -> *const c_char,
) -> Result<&CStr, DWayRenderError> {
    let r = f();
    if r.is_null() {
        if let Some(err) = egl.get_error() {
            Err(DWayRenderError::EglError(err))
        } else {
            Err(DWayRenderError::UnknownEglError)
        }
    } else {
        Ok(unsafe { CStr::from_ptr(r) })
    }
}

pub fn call_gl<R>(gl:&glow::Context,f:impl FnOnce()->R)->Result<R,DWayRenderError>{
    let r=f();
    let err=unsafe{ gl.get_error() };
    if err!=0{
        return Err(DWayRenderError::GLError(err));
    }
    Ok(r)
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
) -> Result<(Vec<T1>, Vec<T2>)> {
    let mut num = 0;
    call_egl_boolean(egl, || f(0, null_mut(), null_mut(), &mut num))?;
    if num == 0 {
        return Ok((vec![], vec![]));
    }
    let mut vec1 = Vec::new();
    vec1.resize_with(num as usize, || Default::default());
    let mut vec2 = Vec::new();
    vec2.resize_with(num as usize, || Default::default());
    call_egl_boolean(egl, || {
        f(
            num,
            vec1.as_mut_ptr() as *mut T1,
            vec2.as_mut_ptr() as *mut T2,
            &mut num,
        )
    })?;
    Ok((vec1, vec2))
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

pub fn get_extensions<E>(f: impl FnOnce() -> Result<String,E>) -> Result<HashSet<String>,E> {
    Ok(f()?
        .split(' ')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string())
        .collect())
}

pub fn get_egl_extensions(egl: &EGLInstance) -> Result<HashSet<String>> {
    Ok(egl
        .query_string(None, khronos_egl::EXTENSIONS)?
        .to_string_lossy()
        .split(' ')
        .filter(|e| !e.is_empty())
        .map(|e| e.to_string())
        .collect())
}

pub fn with_hal<R,FG,FV>(fn_vulkan:FV,fn_gl:FG)-> Result<R,DWayRenderError> where 
FG:FnOnce()-> Result<R,DWayRenderError>,
FV:FnOnce()-> Result<R,DWayRenderError>,
{
    match fn_vulkan(){
        Err(DWayRenderError::BackendIsNotVulkan) => {},
        o =>return o,
    }
    match fn_gl(){
        Err(DWayRenderError::BackendIsNotEGL) => {},
        o =>return o,
    }
    Err(DWayRenderError::UnknownBackend)
}
