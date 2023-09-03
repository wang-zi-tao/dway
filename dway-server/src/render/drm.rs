use super::util::{
    egl_check_extensions, get_egl_function, with_gl, DWayRenderError::*, DEVICE_EXT,
};
use crate::{
    prelude::*,
    util::file::create_sealed_file,
    zwp::dambuffeedback::{do_init_feedback, DmabufFeedback, PeddingDmabufFeedback},
};
use bevy::{
    render::{renderer::RenderDevice, Extract},
    utils::tracing,
};
use crossbeam_channel::{Receiver, Sender};
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use glow::HasContext;
use khronos_egl::{Attrib, Boolean, EGLDisplay, Int};
use nix::libc;
use std::{
    collections::HashSet,
    ffi::{c_char, CStr, CString, OsString},
    fs::File,
    sync::Arc,
};
use thiserror::Error;
use wgpu_hal::{api::Gles, gles::AdapterContext, MemoryFlags, TextureUses};

use super::util::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrmNodeType {
    Primary,
    Control,
    Render,
}
#[derive(Debug, Clone)]
pub struct DrmNode {
    pub device: libc::dev_t,
    pub kind: DrmNodeType,
}
impl DrmNode {
    pub fn new(path: &CStr) -> Result<Self, DWayRenderError> {
        let dev_stat = nix::sys::stat::stat(path).map_err(|e| Unknown(e.into()))?;
        let dev = dev_stat.st_rdev;

        let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0x0000_0fff);
        let minor = ((dev >> 12) & 0xffff_ff00) | ((dev) & 0x0000_00ff);

        let path = format!("/sys/dev/char/{}:{}/device/drm", major, minor);
        if !nix::sys::stat::stat(path.as_str()).is_ok() {
            return Err(NotDrmNode);
        }
        let ty = match minor >> 6 {
            0 => DrmNodeType::Primary,
            1 => DrmNodeType::Control,
            2 => DrmNodeType::Render,
            _ => return Err(NotDrmNode),
        };
        Ok(Self {
            device: dev,
            kind: ty,
        })
    }
}

#[derive(Resource, Default, Debug)]
pub struct DrmNodeState {
    pub state: Option<DrmNodeStateInner>,
}
impl DrmNodeState {
    pub fn init(
        &mut self,
        drm: DrmNode,
        texture_formats: HashSet<DrmFormat>,
        render_formats: HashSet<DrmFormat>,
    ) -> Result<(), DWayRenderError> {
        let texture_format: Vec<DrmFormat> = texture_formats.into_iter().collect();
        let format_table = Self::create_format_table(&texture_format)?;
        let format_indices: Vec<usize> = (0..texture_format.len()).collect();
        let inner = DrmNodeStateInner {
            texture_format,
            render_format: render_formats.into_iter().collect(),
            main_device: drm.clone(),
            main_tranche: DmabufFeedbackTranche {
                target_device: drm,
                flags: zwp_linux_dmabuf_feedback_v1::TrancheFlags::empty(),
                indices: format_indices,
            },
            preferred_tranches: vec![],
            format_table,
        };
        self.state = Some(inner);
        Ok(())
    }
    pub fn create_format_table(texture_format: &Vec<DrmFormat>) -> Result<(File, usize)> {
        let data = texture_format
            .iter()
            .map(|format| (format.code as u32, 0u32, u64::from(format.modifier)))
            .flat_map(|f| bincode::serialize(&f).unwrap())
            .collect::<Vec<_>>();

        Ok(create_sealed_file(
            &CString::new("dway-dmabuffeedback-format-table").unwrap(),
            &data,
        )?)
    }
}
#[derive(Debug)]
pub struct DrmNodeStateInner {
    pub texture_format: Vec<DrmFormat>,
    pub render_format: Vec<DrmFormat>,
    pub main_device: DrmNode,
    pub main_tranche: DmabufFeedbackTranche,
    pub preferred_tranches: Vec<DmabufFeedbackTranche>,
    pub format_table: (File, usize),
}

#[derive(Debug, Clone)]
pub struct DmabufFeedbackTranche {
    pub target_device: DrmNode,
    pub flags: zwp_linux_dmabuf_feedback_v1::TrancheFlags,
    pub indices: Vec<usize>,
}

fn do_init_drm_state(
    device: &wgpu::Device,
    state: &mut DrmNodeState,
) -> Result<(), DWayRenderError> {
    with_gl(device, |context, egl, gl| {
        // egl_check_extensions(
        //     gl,
        //     &[
        //         "EGL_EXT_device_base",
        //         "EGL_EXT_device_query",
        //         "EGL_EXT_device_drm_render_node",
        //         "EGL_EXT_device_drm",
        //     ],
        // )?;
        let extensions = gl.supported_extensions();
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

        let mut device: Attrib = 0;
        call_egl_boolean(egl, || {
            query_display_attrib_ext(egl_display.as_ptr(), DEVICE_EXT, &mut device)
        })?;
        if device == NO_DEVICE_EXT {
            return Err(EglApiError("eglQueryDisplayAttribEXT"));
        }

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
        for format in formats.iter().cloned() {
            let (mods, external) = call_egl_double_vec(egl, |num, vec1, vec2, p_num| {
                query_dma_buf_modifiers_ext(
                    egl_display.as_ptr(),
                    format as i32,
                    num,
                    vec1,
                    vec2,
                    p_num,
                )
            })?;
            if mods.len() == 0 {
                texture_formats.insert(DrmFormat {
                    code: format,
                    modifier: DrmModifier::Invalid,
                });
                render_formats.insert(DrmFormat {
                    code: format,
                    modifier: DrmModifier::Invalid,
                });
            }
            for (modifier, external_only) in mods.into_iter().zip(external.into_iter()) {
                let format = DrmFormat {
                    code: format,
                    modifier: DrmModifier::from(modifier),
                };
                texture_formats.insert(format);
                if external_only == 0 {
                    render_formats.insert(format);
                }
            }
        }
        dbg!(device);
        // let path = call_egl_string(egl, || query_device_string_ext(device, DRM_RENDER_NODE_FILE_EXT))
        //     .or_else(|_| {
        //         egl.get_error();
        //         call_egl_string(egl, || query_device_string_ext(device, DRM_DEVICE_FILE_EXT))
        //     })?;
        let path = CString::new("/dev/dri/renderD128").unwrap(); // TODO 
        dbg!(&path);
        let drm_node = DrmNode::new(&path)?;

        state.init(drm_node, texture_formats, render_formats)?;

        Ok(())
    })
}

#[tracing::instrument(skip_all)]
pub fn init_drm_state(device: Res<RenderDevice>, mut state: ResMut<DrmNodeState>) {
    if state.state.is_some() {
        return;
    }
    if let Err(error) = do_init_drm_state(device.wgpu_device(), &mut state) {
        error!("failed to get drm node info: {error}");
    }
}

pub fn extract_dma_buf_feedback(
    feedback_query: Extract<Query<&DmabufFeedback, With<PeddingDmabufFeedback>>>,
    mut commands: Commands,
) {
    feedback_query.for_each(|feedback| {
        commands.spawn(feedback.clone());
    })
}
#[tracing::instrument(skip_all)]
pub fn init_dma_buf_feedback(
    feedback_query: Query<&DmabufFeedback>,
    drm_node_state: Res<DrmNodeState>,
) {
    let Some(drm_node_state) = &drm_node_state.state else {
        return;
    };
    feedback_query.for_each(|feedback| do_init_feedback(feedback, drm_node_state));
}
