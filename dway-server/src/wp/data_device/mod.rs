pub mod data_source;
pub mod manager;

use crate::{prelude::*, state::add_global_dispatch};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_data_device::WlDataDevice,
}
impl WlDataDevice {
    pub fn new(raw: wl_data_device::WlDataDevice) -> Self {
        Self { raw }
    }
}
relationship!(SelectionOfDataDevice=>SelectionSource--SeatRef);
impl Dispatch<wl_data_device::WlDataDevice, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &wl_data_device::WlDataDevice,
        request: <wl_data_device::WlDataDevice as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_data_device::Request::StartDrag {
                source: _,
                origin: _,
                icon: _,
                serial: _,
            } => {
                warn!("TODO: wl_data_device::Request::StartDrag");
            }
            wl_data_device::Request::SetSelection { source, serial: _ } => {
                if let Some(source) = source {
                    state.connect::<SelectionOfDataDevice>(*data, DWay::get_entity(&source));
                } else {
                    state.disconnect_all::<SelectionOfDataDevice>(*data);
                }
            }
            wl_data_device::Request::Release => {
                state.entity_mut(*data).remove::<WlDataDevice>();
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_data_device::WlDataDevice,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}

pub struct DataDevicePlugin;
impl Plugin for DataDevicePlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wl_data_device_manager::WlDataDeviceManager, 3>(app);
        app.register_relation::<SelectionOfDataDevice>();
    }
}
