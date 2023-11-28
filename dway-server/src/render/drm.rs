use super::util::DWayRenderError::*;
use super::util::*;
use crate::{prelude::*, util::file::create_sealed_file};
use bevy::{render::renderer::RenderDevice, utils::tracing};
use drm_fourcc::DrmFormat;
use nix::libc::{self, dev_t};
use std::{
    ffi::{CStr, CString},
    fs::File,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

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

pub fn new_drm_node_resource() -> (DmaFeedbackWriter, DrmNodeState) {
    let (tx, rx) = channel();
    (
        DmaFeedbackWriter {
            state: None,
            receiver: Mutex::new(rx),
        },
        DrmNodeState {
            state: None,
            sender: tx,
        },
    )
}

pub fn update_dma_feedback_writer(mut dma_feedback_writer: ResMut<DmaFeedbackWriter>) {
    let Ok(new_data) = dma_feedback_writer.receiver.lock().unwrap().try_recv() else {
        return;
    };
    dma_feedback_writer.state = Some(new_data);
}

#[derive(Resource, Debug)]
pub struct DmaFeedbackWriter {
    pub state: Option<Arc<DrmNodeStateInner>>,
    pub receiver: Mutex<Receiver<Arc<DrmNodeStateInner>>>,
}

#[derive(Resource, Debug)]
pub struct DrmNodeState {
    pub state: Option<Arc<DrmNodeStateInner>>,
    pub sender: Sender<Arc<DrmNodeStateInner>>,
}

impl DrmNodeState {
    pub fn set(&mut self, data: DrmNodeStateInner) {
        let data = Arc::new(data);
        self.state = Some(data.clone());
        let _ = self.sender.send(data);
    }

    pub fn init(
        &mut self,
        drm: DrmNode,
        texture_formats: Vec<DrmFormat>,
        render_formats: Vec<DrmFormat>,
    ) -> Result<(), DWayRenderError> {
        let texture_formats: Vec<DrmFormat> = texture_formats.into_iter().collect();
        let format_table = Self::create_format_table(&texture_formats)?;
        let format_indices: Vec<usize> = (0..texture_formats.len()).collect();
        let inner = DrmNodeStateInner {
            texture_formats,
            render_formats,
            main_device: drm.clone(),
            main_tranche: DmabufFeedbackTranche {
                target_device: drm,
                flags: zwp_linux_dmabuf_feedback_v1::TrancheFlags::empty(),
                indices: format_indices,
            },
            preferred_tranches: vec![],
            format_table,
        };
        self.set(inner);
        Ok(())
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

        Ok(create_sealed_file(
            &CString::new("dway-dmabuffeedback-format-table").unwrap(),
            &data,
        )?)
    }
}
#[derive(Debug)]
pub struct DrmNodeStateInner {
    pub texture_formats: Vec<DrmFormat>,
    pub render_formats: Vec<DrmFormat>,
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

#[derive(Debug)]
pub struct DrmInfo {
    pub texture_formats: Vec<DrmFormat>,
    pub render_formats: Vec<DrmFormat>,
    pub drm_node: DrmNode,
}

#[tracing::instrument(skip_all)]
pub fn init_drm_state(device: Res<RenderDevice>, mut state: ResMut<DrmNodeState>) {
    if state.state.is_some() {
        return;
    }
    match with_hal(
        || super::vulkan::drm_info(device.wgpu_device()),
        || super::gles::drm_info(device.wgpu_device()),
    ) {
        Ok(o) => {
            info!("drm info: {o:?}");
            if let Err(e) = state.init(o.drm_node, o.texture_formats, o.render_formats) {
                error!("failed to init drm state: {e}");
            };
        }
        Err(error) => {
            error!("failed to get drm node info: {error}"); // TODO
        }
    }
}
