use wayland_protocols_misc::gtk_primary_selection::server::*;

use crate::{
    clipboard::{ClipboardDataDevice, ClipboardManager, ClipboardSource},
    misc::gtk_primary_selection::source::GtkPrimarySelectionSource,
    prelude::*,
    wp::primary_selection::SourceOfSelection,
};

use super::offer::GtkPrimarySelectionOffer;

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct GtkPrimarySelectionDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: gtk_primary_selection_device::GtkPrimarySelectionDevice,
    #[reflect(ignore, default = "unimplemented")]
    pub dhandle: DisplayHandle,
    pub serial: Option<u32>,
}

impl GtkPrimarySelectionDevice {
    pub fn new(
        raw: gtk_primary_selection_device::GtkPrimarySelectionDevice,
        dhandle: DisplayHandle,
    ) -> Self {
        Self {
            raw,
            dhandle,
            serial: None,
        }
    }
}
impl Drop for GtkPrimarySelectionDevice {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<gtk_primary_selection_device::GtkPrimarySelectionDevice, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &gtk_primary_selection_device::GtkPrimarySelectionDevice,
        request: <gtk_primary_selection_device::GtkPrimarySelectionDevice as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);

        state.with_component_mut(resource, |c: &mut GtkPrimarySelectionDevice| {
            c.serial = None;
        });

        match request {
            gtk_primary_selection_device::Request::SetSelection { source, serial } => {
                if let Some(source) = source {
                    state.connect::<SourceOfSelection>(*data, DWay::get_entity(&source));
                    let mime_types = state
                        .object_component::<GtkPrimarySelectionSource>(&source)
                        .mime_types
                        .clone();
                    ClipboardManager::add_source(
                        state.world_mut(),
                        ClipboardSource::GtkPrimarySelectionSource(source.clone()),
                        mime_types,
                    );
                } else {
                    state.disconnect_all::<SourceOfSelection>(*data);
                }
            }
            gtk_primary_selection_device::Request::Destroy => {
                state.despawn_object(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &gtk_primary_selection_device::GtkPrimarySelectionDevice,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

impl ClipboardDataDevice for GtkPrimarySelectionDevice {
    fn create_offer(&self, mime_types: &Vec<String>, mut commands: Commands) {
        let self_entity = DWay::get_entity(&self.raw);
        let Some(client) = self.raw.client() else {
            return;
        };

        match GtkPrimarySelectionOffer::create(
            &self.dhandle,
            &client,
            self.raw.version(),
            self_entity,
        ) {
            Ok(data_offer) => {
                let raw = data_offer.raw.clone();
                commands.entity(self_entity).insert(data_offer);

                self.raw.data_offer(&raw);
                for mime_type in mime_types {
                    raw.offer(mime_type.clone());
                }
                self.raw.selection(Some(&raw));
            }
            Err(e) => {
                error!("failed to create WlDataOffer: {e}");
            }
        };
    }
}
