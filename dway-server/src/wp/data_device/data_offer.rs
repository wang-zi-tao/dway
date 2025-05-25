use bevy::ecs::system::SystemState;
use wl_data_device_manager::DndAction;

use crate::{
    clipboard::{ClipboardManager, ClipboardRecord, DataOffer, MimeTypeSet, PasteRequest},
    prelude::*,
    wp::data_device::data_source::WlDataSource,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataOffer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_data_offer::WlDataOffer,
    active: bool,
    dropped: bool,
    accepted: bool,
    chosen_action: DndAction,
}

impl WlDataOffer {
}

impl WlDataOffer {
    pub fn new(raw: wl_data_offer::WlDataOffer) -> Self {
        Self {
            raw,
            active: true,
            dropped: false,
            accepted: false,
            chosen_action: DndAction::None,
        }
    }

    pub fn create(
        dh: &DisplayHandle,
        client: &wayland_server::Client,
        version: u32,
        entity: Entity,
    ) -> anyhow::Result<Self> {
        let raw = client
            .create_resource::<wl_data_offer::WlDataOffer, Entity, DWay>(dh, version, entity)?;
        Ok(Self::new(raw))
    }
}
impl Drop for WlDataOffer {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<wl_data_offer::WlDataOffer, Entity> for DWay {
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_data_offer::WlDataOffer,
        request: <wl_data_offer::WlDataOffer as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        let mut system_state = SystemState::<(
            Query<(&mut WlDataOffer, &mut WlDataSource)>,
            ResMut<ClipboardManager>,
        )>::from_world(state.world_mut());
        let (mut query, mut clipboard_manager) = system_state.get_manual_mut(state.world_mut());
        let Ok((mut data_offer, data_source)) = query.get_mut(*data) else {
            return;
        };
        match request {
            wl_data_offer::Request::Accept { serial, mime_type } => {
                if let Some(mtype) = &mime_type {
                    data_offer.accepted = data_source.mime_types.contains(mtype);
                } else {
                    data_offer.accepted = false;
                }
            }
            wl_data_offer::Request::Receive { mime_type, fd } => {
                ClipboardManager::require_last_record(
                    state.world_mut(),
                    PasteRequest {
                        mime_type,
                        fd,
                        data_offer: DataOffer::WlDataOffer(resource.clone()),
                    },
                );
            }
            wl_data_offer::Request::Destroy => {
                state.destroy_object(resource);
            }
            wl_data_offer::Request::Finish => {
                data_offer.active = false;
            }
            wl_data_offer::Request::SetActions {
                dnd_actions,
                preferred_action,
            } => {
                todo!()
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wayland_server::protocol::wl_data_offer::WlDataOffer,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<WlDataOffer>(*data, resource);
    }
}
