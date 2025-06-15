use bevy::ecs::system::SystemState;
use wl_data_device_manager::DndAction;

use crate::{
    clipboard::{ClipboardManager, ClipboardRecord, DataOffer, MimeTypeSet, PasteRequest},
    prelude::*,
    wp::data_device::{
        data_source::WlDataSource,
        dnd::{DragAndDrop, DropFrom},
    },
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct DropData {
    pub active: bool,
    pub dropped: bool,
    pub accepted: bool,
    pub chosen_action: DndAction,
}

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
        match request {
            wl_data_offer::Request::Accept { serial, mime_type } => {
                let accepted = if let Some(source) = state
                    .get::<DropFrom>(*data)
                    .and_then(|d| d.get())
                    .and_then(|dnd_entity| state.get::<DragAndDrop>(dnd_entity))
                    .and_then(|dnd| dnd.data_source)
                    .and_then(|source_entity| state.get::<WlDataSource>(source_entity))
                {
                    mime_type
                        .map(|mime_type| source.mime_types.contains(&*mime_type))
                        .unwrap_or(false)
                } else {
                    false
                };
                if let Some(mut drop_data) = state.get_mut::<DropData>(*data) {
                    drop_data.accepted = accepted;
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
                let Some(mut data_offer) = state.get_mut::<WlDataOffer>(*data) else {
                    return;
                };
                data_offer.active = false;
            }
            wl_data_offer::Request::SetActions {
                dnd_actions,
                preferred_action,
            } => {
                let dnd_actions = dnd_actions.into_result().unwrap_or(DndAction::None);
                let preferred_action = preferred_action.into_result().unwrap_or(DndAction::None);

                if let Some(source) = state
                    .get::<DropFrom>(*data)
                    .and_then(|d| d.get())
                    .and_then(|dnd_entity| state.get::<DragAndDrop>(dnd_entity))
                    .and_then(|dnd| dnd.data_source)
                    .and_then(|source_entity| state.get::<WlDataSource>(source_entity))
                {
                    let sourcec_action = source.dnd_action;
                    let data_source = source.raw.clone();

                    let chosen_action =
                        DragAndDrop::choise_action(sourcec_action & dnd_actions, preferred_action);

                    debug!("choise action {:?}", chosen_action);

                    if let Some(mut drop_data) = state.get_mut::<DropData>(*data) {
                        if drop_data.chosen_action != chosen_action {
                            drop_data.chosen_action = chosen_action;
                            data_source.action(chosen_action);
                            resource.action(chosen_action);
                        }
                    }
                }
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
