use std::{
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use crate::{
    prelude::*,
    render::{drm::DmaBackend, DWayServerRenderClient},
};

#[derive(Debug, Default)]
pub struct DmabufFeedbackInner {
    pub inited: bool,
}
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmabufFeedback {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
    #[reflect(ignore)]
    pub inner: Arc<Mutex<DmabufFeedbackInner>>,
}
impl DmabufFeedback {
    pub fn new(raw: zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1) -> Self {
        Self {
            raw,
            inner: Default::default(),
        }
    }
}
impl Dispatch<zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        request: <zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data, resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_linux_dmabuf_feedback_v1::Request::Destroy => {
                state.despawn_object_component::<DmabufFeedback>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<DmabufFeedback>(*data, resource);
    }
}

pub fn do_init_feedback(
    feedback: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
    dma_backend: &DmaBackend,
) {
    feedback.main_device(
        dma_backend
            .main_tranche
            .target_device
            .device
            .to_ne_bytes()
            .to_vec(),
    );
    feedback.format_table(
        dma_backend.format_table.0.as_fd(),
        dma_backend.format_table.1 as u32,
    );

    for tranche in dma_backend
        .preferred_tranches
        .iter()
        .chain(std::iter::once(&dma_backend.main_tranche))
    {
        feedback.tranche_target_device(tranche.target_device.device.to_ne_bytes().to_vec());
        feedback.tranche_flags(tranche.flags);
        feedback.tranche_formats(
            tranche
                .indices
                .iter()
                .flat_map(|i| (*i as u16).to_ne_bytes())
                .collect::<Vec<_>>(),
        );
        feedback.tranche_done();
    }

    feedback.done();
    debug!(resource=%feedback.id(),"init dma buffer feedback");
}

pub fn init_feedback(
    world: &mut World,
    feedback: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
) {
    if let Some(drm_node) = &world
        .resource::<DWayServerRenderClient>()
        .drm_node
        .lock()
        .ok()
    {
        if let Some(drm_node) = drm_node.as_ref() {
            do_init_feedback(feedback, drm_node)
        }
    } else {
        warn!("failed to init dmabuf feedback");
    }
}
