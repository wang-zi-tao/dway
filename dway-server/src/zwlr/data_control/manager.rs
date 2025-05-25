use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_manager_v1::*;

use crate::{
    clipboard::ClipboardManager,
    prelude::*,
    state::add_global_dispatch,
    zwlr::data_control::{
        source::ZwlrDataControlSource, device::ZwlrDataControlDevice,
        offer::ZwlrDataControlOffer,
    },
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwlrDataControlManager {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwlrDataControlManagerV1,
}
impl ZwlrDataControlManager {
    pub fn new(raw: ZwlrDataControlManagerV1) -> Self {
        Self { raw }
    }
}
impl Drop for ZwlrDataControlManager {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<ZwlrDataControlManagerV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &ZwlrDataControlManagerV1,
        request: <ZwlrDataControlManagerV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            Request::CreateDataSource { id } => {
                state.spawn_child_object(*data, id, data_init, |o| ZwlrDataControlSource::new(o));
            }
            Request::GetDataDevice { id, seat } => {
                let entity = state
                    .spawn_child_object(*data, id, data_init, |o| ZwlrDataControlDevice::new(o));

                let mime_types =
                    ClipboardManager::get_mime_types(state.world()).unwrap_or_default();

                if let Ok(mut entity_mut) = state.get_entity_mut(entity) {
                    let data_device = entity_mut
                        .get::<ZwlrDataControlDevice>()
                        .unwrap()
                        .raw
                        .clone();

                    match ZwlrDataControlOffer::create(
                        dhandle,
                        client,
                        data_device.version(),
                        entity,
                    ) {
                        Ok(data_offer) => {
                            let data_offer_raw = data_offer.raw.clone();
                            entity_mut.insert(data_offer);

                            data_device.data_offer(&data_offer_raw);
                            for mime_type in mime_types {
                                data_offer_raw.offer(mime_type);
                            }
                            data_device.selection(Some(&data_offer_raw));
                        }
                        Err(e) => {
                            error!("failed to create WlDataOffer: {e}");
                        }
                    };
                }
            }
            Request::Destroy => {
                state.despawn_object_component::<ZwlrDataControlManager>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwlrDataControlManagerV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<ZwlrDataControlManager>(*data, resource);
    }
}

impl GlobalDispatch<ZwlrDataControlManagerV1, Entity> for DWay {
    fn bind(
        state: &mut DWay,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<ZwlrDataControlManagerV1>,
        global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            ZwlrDataControlManager::new(o)
        });
    }
}
