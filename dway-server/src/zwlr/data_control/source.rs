use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_source_v1::*;

use crate::{clipboard::MimeTypeSet, prelude::*};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwlrDataControlSource {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwlrDataControlSourceV1,
    pub mime_types: MimeTypeSet,
}
impl ZwlrDataControlSource {
    pub fn new(raw: ZwlrDataControlSourceV1) -> Self {
        Self { raw, mime_types: default() }
    }
}
impl Drop for ZwlrDataControlSource {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<ZwlrDataControlSourceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &ZwlrDataControlSourceV1,
        request: <ZwlrDataControlSourceV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);

        match request {
            Request::Offer { mime_type } => {
                if let Some(mut c) = state.get_mut::<ZwlrDataControlSource>(*data) {
                    c.mime_types.insert(mime_type);
                }
            },
            Request::Destroy => {
                state.despawn_object_component::<ZwlrDataControlSource>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwlrDataControlSourceV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<ZwlrDataControlSource>(*data, resource);
    }
}
