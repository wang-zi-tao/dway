use std::{
    os::fd::AsRawFd,
    sync::{Arc, Mutex},
};

use crate::{
    prelude::*,
    render::drm::{DrmNodeState, DrmNodeStateInner},
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct PeddingDmabufFeedback;

#[derive(Debug, Default)]
pub struct DmabufFeedbackInner {
    pub inited: bool,
}
#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmabufFeedback {
    #[reflect(ignore)]
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
        client: &wayland_server::Client,
        resource: &zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
        request: <zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
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
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<DmabufFeedback>(*data, resource);
    }
}

pub fn do_init_feedback(feedback: &DmabufFeedback, drm_node_state: &DrmNodeStateInner) {
    let mut guard = feedback.inner.lock().unwrap();
    if guard.inited {
        return;
    }
    let raw = &feedback.raw;
    dbg!(&drm_node_state.format_table);
    raw.main_device(drm_node_state.main_device.device.to_ne_bytes().to_vec());
    raw.format_table(
        drm_node_state.format_table.0.as_raw_fd(),
        drm_node_state.format_table.1 as u32,
    );

    for tranche in drm_node_state
        .preferred_tranches
        .iter()
        .chain(std::iter::once(&drm_node_state.main_tranche))
    {
        raw.tranche_target_device(tranche.target_device.device.to_ne_bytes().to_vec());
        raw.tranche_flags(tranche.flags);
        raw.tranche_formats(
            tranche
                .indices
                .iter()
                .flat_map(|i| (*i as u16).to_ne_bytes())
                .collect::<Vec<_>>(),
        );
        raw.tranche_done();
    }

    debug!(resource=%feedback.raw.id(),"init dma buffer feedback");
    raw.done();
    guard.inited = true;
}

pub fn update_feedback_state(
    feedback_query: Query<(Entity, &DmabufFeedback), With<PeddingDmabufFeedback>>,
    mut commands: Commands,
) {
    feedback_query.for_each(|(entity, feedback)| {
        if feedback.inner.lock().unwrap().inited {
            commands.entity(entity).remove::<PeddingDmabufFeedback>();
        }
    })
}
