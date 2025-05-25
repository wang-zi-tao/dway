use crate::{
    clipboard::{ClipboardDataDevice, ClipboardManager},
    prelude::*,
    wp::data_device::{data_offer::WlDataOffer, data_source::WlDataSource, WlDataDevice},
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataDeviceManager {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_data_device_manager::WlDataDeviceManager,
}
impl WlDataDeviceManager {
    pub fn new(raw: wl_data_device_manager::WlDataDeviceManager) -> Self {
        Self { raw }
    }
}
impl Dispatch<wl_data_device_manager::WlDataDeviceManager, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wl_data_device_manager::WlDataDeviceManager,
        request: <wl_data_device_manager::WlDataDeviceManager as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_data_device_manager::Request::CreateDataSource { id } => {
                state.spawn_child_object(*data, id, data_init, WlDataSource::new);
            }
            wl_data_device_manager::Request::GetDataDevice { id, seat } => {
                let seat_entity = DWay::get_entity(&seat);
                let device_entity = state.spawn_child_object(seat_entity, id, data_init, |o| {
                    WlDataDevice::new(o, dhandle.clone())
                });

                WlDataDevice::init_data_device(device_entity, state.world_mut());
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_data_device_manager::WlDataDeviceManager,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl GlobalDispatch<wl_data_device_manager::WlDataDeviceManager, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_data_device_manager::WlDataDeviceManager>,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, WlDataDeviceManager::new);
    }
}
