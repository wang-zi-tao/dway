pub mod data_offer;
pub mod data_source;
pub mod manager;

use data_offer::WlDataOffer;
use data_source::WlDataSource;

use crate::{
    clipboard::{
        send_selection_system, ClipboardDataDevice, ClipboardEvent, ClipboardManager,
        ClipboardSource, MimeTypeSet,
    },
    prelude::*,
    schedule::DWayServerSchedule,
    state::add_global_dispatch,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_data_device::WlDataDevice,
    #[reflect(ignore, default = "unimplemented")]
    pub dhandle: DisplayHandle,
}
impl WlDataDevice {
    pub fn new(raw: wl_data_device::WlDataDevice, dhandle: DisplayHandle) -> Self {
        Self { raw, dhandle }
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
                if let Some(source) = &source {
                    state.connect::<SelectionOfDataDevice>(*data, DWay::get_entity(source));

                    let mime_types = state
                        .object_component::<WlDataSource>(source)
                        .mime_types
                        .clone();
                    ClipboardManager::add_source(
                        state.world_mut(),
                        ClipboardSource::DataDevice(source.clone()),
                        mime_types,
                    );
                } else {
                    state.disconnect_all::<SelectionOfDataDevice>(*data);
                }
            }
            wl_data_device::Request::Release => {
                state.despawn_object(*data, resource);
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

impl ClipboardDataDevice for WlDataDevice {
    fn create_offer(&self, mime_types: &Vec<String>, mut commands: Commands) {
        let self_entity = DWay::get_entity(&self.raw);
        let Some(client) = self.raw.client() else {
            return;
        };
        match WlDataOffer::create(&self.dhandle, &client, self.raw.version(), self_entity) {
            Ok(data_offer) => {
                let raw = data_offer.raw.clone();
                commands.entity(self_entity).insert(data_offer);

                self.raw.data_offer(&raw);
                for mime_type in mime_types.iter() {
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

pub struct DataDevicePlugin;
impl Plugin for DataDevicePlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wl_data_device_manager::WlDataDeviceManager, 3>(app);
        app.register_relation::<SelectionOfDataDevice>();
        app.init_resource::<ClipboardManager>();
        app.add_systems(
            PreUpdate,
            (ClipboardManager::receive_data_system, send_selection_system)
                .in_set(DWayServerSet::UpdateClipboard),
        );
        app.add_event::<ClipboardEvent>();
    }
}
