use bevy::platform::collections::HashSet;
use wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_source_v1::{
    self, ZwpPrimarySelectionSourceV1,
};

use crate::{clipboard::MimeTypeSet, prelude::*};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct PrimarySelectionSource {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwpPrimarySelectionSourceV1,
    pub mime_types: MimeTypeSet,
}
impl PrimarySelectionSource {
    pub fn new(raw: ZwpPrimarySelectionSourceV1) -> Self {
        Self {
            raw,
            mime_types: default(),
        }
    }
}
impl Dispatch<ZwpPrimarySelectionSourceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &ZwpPrimarySelectionSourceV1,
        request: <ZwpPrimarySelectionSourceV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_primary_selection_source_v1::Request::Offer { mime_type } => {
                state.with_component_mut(resource, |c: &mut PrimarySelectionSource| {
                    c.mime_types.insert(mime_type);
                });
            }
            zwp_primary_selection_source_v1::Request::Destroy => {
                state.despawn(*data);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwpPrimarySelectionSourceV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
