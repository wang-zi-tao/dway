use wayland_protocols_misc::gtk_primary_selection::server::*;

use crate::{clipboard::MimeTypeSet, prelude::*};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct GtkPrimarySelectionSource {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: gtk_primary_selection_source::GtkPrimarySelectionSource,
    pub mime_types: MimeTypeSet,
}
impl GtkPrimarySelectionSource {
    pub fn new(raw: gtk_primary_selection_source::GtkPrimarySelectionSource) -> Self {
        Self {
            raw,
            mime_types: default(),
        }
    }
}
impl Drop for GtkPrimarySelectionSource {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<gtk_primary_selection_source::GtkPrimarySelectionSource, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &gtk_primary_selection_source::GtkPrimarySelectionSource,
        request: <gtk_primary_selection_source::GtkPrimarySelectionSource as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);

        match request {
            gtk_primary_selection_source::Request::Offer { mime_type } => {
                state.with_component_mut(resource, |c: &mut GtkPrimarySelectionSource| {
                    c.mime_types.insert(mime_type);
                });
            }
            gtk_primary_selection_source::Request::Destroy => {
                state.despawn_object(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &gtk_primary_selection_source::GtkPrimarySelectionSource,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
