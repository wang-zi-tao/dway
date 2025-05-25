use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_offer_v1::*;

use crate::{
    clipboard::{ClipboardManager, DataOffer, PasteRequest},
    prelude::*,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwlrDataControlOffer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwlrDataControlOfferV1,
}
impl ZwlrDataControlOffer {
    pub fn create(
        dh: &DisplayHandle,
        client: &wayland_server::Client,
        version: u32,
        entity: Entity,
    ) -> anyhow::Result<Self> {
        let raw =
            client.create_resource::<ZwlrDataControlOfferV1, Entity, DWay>(dh, version, entity)?;
        Ok(Self::new(raw))
    }

    pub fn new(raw: ZwlrDataControlOfferV1) -> Self {
        Self { raw }
    }
}
impl Drop for ZwlrDataControlOffer {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<ZwlrDataControlOfferV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &ZwlrDataControlOfferV1,
        request: <ZwlrDataControlOfferV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);

        match request {
            Request::Receive { mime_type, fd } => {
                ClipboardManager::require_last_record(
                    state.world_mut(),
                    PasteRequest {
                        mime_type,
                        fd,
                        data_offer: DataOffer::ZwlrDataControlOffer(resource.clone()),
                    },
                );
            }
            Request::Destroy => {
                state.despawn_object_component::<ZwlrDataControlOffer>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwlrDataControlOfferV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<ZwlrDataControlOffer>(*data, resource);
    }
}
