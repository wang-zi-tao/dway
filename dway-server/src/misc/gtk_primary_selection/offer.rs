use wayland_protocols_misc::gtk_primary_selection::server::*;

use crate::{
    clipboard::{ClipboardManager, DataOffer, PasteRequest},
    prelude::*,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct GtkPrimarySelectionOffer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: gtk_primary_selection_offer::GtkPrimarySelectionOffer,
}
impl GtkPrimarySelectionOffer {
    pub fn new(raw: gtk_primary_selection_offer::GtkPrimarySelectionOffer) -> Self {
        Self { raw }
    }

    pub fn create(
        dh: &DisplayHandle,
        client: &wayland_server::Client,
        version: u32,
        entity: Entity,
    ) -> anyhow::Result<Self> {
        let raw = client
            .create_resource::<gtk_primary_selection_offer::GtkPrimarySelectionOffer, Entity, DWay>(
                dh, version, entity,
            )?;
        Ok(Self::new(raw))
    }
}
impl Drop for GtkPrimarySelectionOffer {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<gtk_primary_selection_offer::GtkPrimarySelectionOffer, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &gtk_primary_selection_offer::GtkPrimarySelectionOffer,
        request: <gtk_primary_selection_offer::GtkPrimarySelectionOffer as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);

        match request {
            gtk_primary_selection_offer::Request::Receive { mime_type, fd } => {
                ClipboardManager::require_last_record(
                    state.world_mut(),
                    PasteRequest {
                        mime_type,
                        fd,
                        data_offer: DataOffer::GtkPrimarySelectionOffer(resource.clone()),
                    },
                );
            }
            gtk_primary_selection_offer::Request::Destroy => {
                state.despawn_object_component::<GtkPrimarySelectionOffer>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &gtk_primary_selection_offer::GtkPrimarySelectionOffer,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<GtkPrimarySelectionOffer>(*data, resource);
    }
}
