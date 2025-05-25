use super::{
    device::PrimarySelectionDevice, offer::ZwpPrimarySelectionOffer, source::PrimarySelectionSource,
};
use crate::{
    clipboard::{ClipboardDataDevice, ClipboardManager},
    prelude::*,
};

#[derive(Component)]
pub struct PrimarySelectionDeviceManager {
    pub raw: zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
}

pub struct ZwpPrimarySelectionDeviceManagerV1 {
    pub raw: zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
}

impl Dispatch<zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1, Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        request: <zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            zwp_primary_selection_device_manager_v1::Request::CreateSource { id } => {
                state.spawn_child_object(*data, id, data_init, PrimarySelectionSource::new);
            }
            zwp_primary_selection_device_manager_v1::Request::GetDevice { id, seat } => {
                let device_entity = state.spawn_child_object(*data, id, data_init, |o| {
                    PrimarySelectionDevice::new(o, dhandle.clone())
                });

                PrimarySelectionDevice::init_data_device(device_entity, state.world_mut());
            }
            zwp_primary_selection_device_manager_v1::Request::Destroy => {
                state.despawn_object_component::<PrimarySelectionDeviceManager>(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<PrimarySelectionDeviceManager>(*data, resource);
    }
}

impl
    GlobalDispatch<
        zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        Entity,
    > for DWay
{
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1>,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            PrimarySelectionDeviceManager { raw: o }
        });
    }
}
