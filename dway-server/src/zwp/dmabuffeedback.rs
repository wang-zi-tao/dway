use std::{
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use crate::{
    prelude::*,
    render::drm::{DmaFeedbackWriter, DrmNodeStateInner},
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
                state.entity_mut(*data).remove::<DmabufFeedback>();
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
    drm_node_state: &DrmNodeStateInner,
) {
    feedback.main_device(drm_node_state.main_device.device.to_ne_bytes().to_vec());
    feedback.format_table(
        drm_node_state.format_table.0.as_fd(),
        drm_node_state.format_table.1 as u32,
    );

    for tranche in drm_node_state
        .preferred_tranches
        .iter()
        .chain(std::iter::once(&drm_node_state.main_tranche))
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
    if let Some(drm_node_state) = &world.resource::<DmaFeedbackWriter>().state {
        do_init_feedback(feedback, drm_node_state)
    }else{
        warn!("failed to init dmabuf feedback");
    }
}
