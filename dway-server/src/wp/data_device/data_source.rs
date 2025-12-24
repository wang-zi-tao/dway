use std::collections::HashSet;

use bevy_relationship::reexport::SmallVec;
use wayland_server::protocol::wl_data_device_manager::DndAction;

use crate::{clipboard::MimeTypeSet, prelude::*};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataSource {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_data_source::WlDataSource,
    pub mime_types: MimeTypeSet,
    #[reflect(ignore, default = "unimplemented")]
    pub dnd_action: DndAction,
}
impl WlDataSource {
    pub fn new(raw: wl_data_source::WlDataSource) -> Self {
        Self {
            raw,
            mime_types: Default::default(),
            dnd_action: DndAction::None,
        }
    }
}
impl Dispatch<wl_data_source::WlDataSource, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &wl_data_source::WlDataSource,
        request: <wl_data_source::WlDataSource as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_data_source::Request::Offer { mime_type } => {
                state.with_component_mut(resource, |c: &mut WlDataSource| {
                    c.mime_types.insert(mime_type);
                });
            }
            wl_data_source::Request::Destroy => {
                state.destroy_object(resource);
            }
            wl_data_source::Request::SetActions { dnd_actions } => {
                match dnd_actions {
                    WEnum::Value(action) => {
                        state.with_component_mut(resource, |c: &mut WlDataSource| {
                            c.dnd_action = action;
                        });
                    }
                    WEnum::Unknown(action) => {
                        warn!("Unknown dnd_action: {:?}", action);
                    }
                    _ => {}
                };
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_data_source::WlDataSource,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
