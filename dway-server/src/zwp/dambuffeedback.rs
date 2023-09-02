use crate::prelude::*;

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmabufFeedback {
    #[reflect(ignore)]
    pub raw: zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1,
}
impl DmabufFeedback {
    pub fn new(raw: zwp_linux_dmabuf_feedback_v1::ZwpLinuxDmabufFeedbackV1) -> Self {
        Self { raw }
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
