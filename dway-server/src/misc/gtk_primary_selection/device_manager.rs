use wayland_protocols_misc::gtk_primary_selection::server::*;

use crate::{clipboard::ClipboardDataDevice, misc::gtk_primary_selection::{device::GtkPrimarySelectionDevice, source::GtkPrimarySelectionSource}, prelude::*};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct GtkPrimarySelectionDeviceManager {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
}
impl GtkPrimarySelectionDeviceManager {
    pub fn new(
        raw: gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
    ) -> Self {
        Self { raw }
    }
}
impl Drop for GtkPrimarySelectionDeviceManager {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager, Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
        request: <gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            gtk_primary_selection_device_manager::Request::CreateSource { id } => {
                state.spawn_child_object(*data, id, data_init, GtkPrimarySelectionSource::new);
            }
            gtk_primary_selection_device_manager::Request::GetDevice { id, seat: _ } => {
                let device_entity = state.spawn_child_object(*data, id, data_init, |o| {
                    GtkPrimarySelectionDevice::new(o, dhandle.clone())
                });

                GtkPrimarySelectionDevice::init_data_device(device_entity, state.world_mut());
            }
            gtk_primary_selection_device_manager::Request::Destroy => {
                state.despawn_object_component::<GtkPrimarySelectionDeviceManager>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<GtkPrimarySelectionDeviceManager>(*data, resource);
    }
}
impl GlobalDispatch<gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager, Entity>
    for DWay
{
    fn bind(
        state: &mut DWay,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<
            gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
        >,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            GtkPrimarySelectionDeviceManager::new(o)
        });
    }
}
