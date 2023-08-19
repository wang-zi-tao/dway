use crate::prelude::*;

use super::{source::PrimarySelectionSource, PrimarySelectionDevice};

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
        _client: &wayland_server::Client,
        _resource: &zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
        request: <zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            zwp_primary_selection_device_manager_v1::Request::CreateSource { id } => {
                state.spawn_child_object(*data, id, data_init, PrimarySelectionSource::new);
            }
            zwp_primary_selection_device_manager_v1::Request::GetDevice { id, seat: _ } => {
                state.spawn_child_object(*data, id, data_init, PrimarySelectionDevice::new);
            }
            zwp_primary_selection_device_manager_v1::Request::Destroy => todo!(),
            _ => todo!(),
        }
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
