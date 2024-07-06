use std::{
    ffi::{CStr, CString},
    fs::File,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use bevy::render::renderer::RenderDevice;
use drm_fourcc::DrmFormat;
use nix::libc::{self, dev_t};

use super::{
    util::{DWayRenderError::*, *},
    DWayServerRenderServer,
};
use crate::{prelude::*, util::file::create_sealed_file};

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
        let dev_stat = nix::sys::stat::stat(path)
            .map_err(|e| Unknown(anyhow!("failed to open drm device: {:?} : {}", path, e)))?;
        Self::from_device_id(dev_stat.st_rdev)
    }

    pub fn from_device_id(dev: dev_t) -> Result<Self, DWayRenderError> {
        let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0x0000_0fff);
        let minor = ((dev >> 12) & 0xffff_ff00) | ((dev) & 0x0000_00ff);

        let path = format!("/sys/dev/char/{}:{}/device/drm", major, minor);
        if nix::sys::stat::stat(path.as_str()).is_err() {
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

#[derive(Debug)]
pub struct DmaBackend {
    pub texture_formats: Vec<DrmFormat>,
    pub render_formats: Vec<DrmFormat>,
    pub main_tranche: DmabufFeedbackTranche,
    pub preferred_tranches: Vec<DmabufFeedbackTranche>,
    pub format_table: (File, usize),
}

impl DmaBackend {
    pub fn new(drm: DrmInfo) -> Result<Self, DWayRenderError> {
        let DrmInfo {
            texture_formats,
            render_formats,
            drm_node,
        } = drm;
        let texture_formats: Vec<DrmFormat> = texture_formats.into_iter().collect();
        let format_table = create_format_table(&texture_formats)?;
        let format_indices: Vec<usize> = (0..texture_formats.len()).collect();
        Ok(DmaBackend {
            texture_formats,
            render_formats,
            main_tranche: DmabufFeedbackTranche {
                target_device: drm_node,
                flags: zwp_linux_dmabuf_feedback_v1::TrancheFlags::empty(),
                indices: format_indices,
            },
            preferred_tranches: vec![],
            format_table,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DmabufFeedbackTranche {
    pub target_device: DrmNode,
    pub flags: zwp_linux_dmabuf_feedback_v1::TrancheFlags,
    pub indices: Vec<usize>,
}

pub fn create_format_table(texture_format: &Vec<DrmFormat>) -> Result<(File, usize)> {
    if texture_format.is_empty() {
        warn!("invalid drm format table");
    }
    let data = texture_format
        .iter()
        .map(|format| (format.code as u32, 0u32, u64::from(format.modifier)))
        .flat_map(|f| bincode::serialize(&f).unwrap())
        .collect::<Vec<_>>();

    create_sealed_file(
        &CString::new("dway-dmabuffeedback-format-table").unwrap(),
        &data,
    )
}

#[derive(Debug)]
pub struct DrmInfo {
    pub texture_formats: Vec<DrmFormat>,
    pub render_formats: Vec<DrmFormat>,
    pub drm_node: DrmNode,
}

#[tracing::instrument(skip_all)]
pub fn init_drm_state(device: Res<RenderDevice>, mut server: Res<DWayServerRenderServer>) {
    let Ok(mut guard) = server.drm_node.lock() else {
        return;
    };
    if guard.is_some() {
        return;
    }
    info_span!("update drm node state");
    match with_hal(
        || super::vulkan::drm_info(device.wgpu_device()),
        || super::gles::drm_info(device.wgpu_device()),
    ) {
        Ok(drm_info) => {
            info!("drm info: {drm_info:?}");
            match DmaBackend::new(drm_info) {
                Err(e) => {
                    error!("failed to init drm state: {e}");
                    return;
                }
                Ok(state_inner) => {
                    *guard = Some(state_inner);
                }
            };
        }
        Err(error) => {
            error!("failed to get drm node info: {error}"); // TODO
        }
    }
}
