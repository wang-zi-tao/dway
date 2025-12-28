use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_device_v1::*;

use super::offer::ZwlrDataControlOffer;
use crate::{
    clipboard::{ClipboardDataDevice, ClipboardManager, ClipboardSource},
    prelude::*,
    wp::primary_selection::SourceOfSelection,
    zwlr::data_control::source::ZwlrDataControlSource,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwlrDataControlDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwlrDataControlDeviceV1,
    #[reflect(ignore, default = "unimplemented")]
    pub dhandle: DisplayHandle,
}
impl ZwlrDataControlDevice {
    pub fn new(raw: ZwlrDataControlDeviceV1, dhandle: DisplayHandle) -> Self {
        Self { raw, dhandle }
    }
}
impl Drop for ZwlrDataControlDevice {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<ZwlrDataControlDeviceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &ZwlrDataControlDeviceV1,
        request: <ZwlrDataControlDeviceV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            Request::SetSelection { source } => {
                if let Some(source) = source {
                    state.connect::<SourceOfSelection>(*data, DWay::get_entity(&source));

                    let mime_types = state
                        .object_component::<ZwlrDataControlSource>(&source)
                        .mime_types
                        .clone();
                    ClipboardManager::add_source(
                        state.world_mut(),
                        ClipboardSource::DataControlSource(source.clone()),
                        mime_types,
                    );
                } else {
                    state.disconnect_all::<SourceOfSelection>(*data);
                }
            }
            Request::Destroy => {
                state.despawn_object(*data, resource);
            }
            Request::SetPrimarySelection { source: _ } => todo!(),
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwlrDataControlDeviceV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

impl ClipboardDataDevice for ZwlrDataControlDevice {
    fn create_offer(&self, mime_types: &Vec<String>, mut commands: Commands) {
        let self_entity = DWay::get_entity(&self.raw);
        let Some(client) = self.raw.client() else {
            return;
        };
        match ZwlrDataControlOffer::create(&self.dhandle, &client, self.raw.version(), self_entity)
        {
            Ok(data_offer) => {
                let data_offer_raw = data_offer.raw.clone();
                commands.entity(self_entity).insert(data_offer);

                self.raw.data_offer(&data_offer_raw);
                for mime_type in mime_types {
                    data_offer_raw.offer(mime_type.clone());
                }
                self.raw.selection(Some(&data_offer_raw));
            }
            Err(e) => {
                error!("failed to create WlDataOffer: {e}");
            }
        };
    }
}
