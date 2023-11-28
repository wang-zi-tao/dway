use crate::{prelude::*, state::add_global_dispatch};

#[derive(Component)]
pub struct DrmLeaseDevice {
    pub raw: wp_drm_lease_device_v1::WpDrmLeaseDeviceV1,
}

impl wayland_server::Dispatch<wp_drm_lease_request_v1::WpDrmLeaseRequestV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wp_drm_lease_request_v1::WpDrmLeaseRequestV1,
        request: <wp_drm_lease_request_v1::WpDrmLeaseRequestV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }

    fn destroyed(
        _state: &mut Self,
        _client: wayland_backend::server::ClientId,
        _resource: &wp_drm_lease_request_v1::WpDrmLeaseRequestV1,
        _data: &Entity,
    ) {
    }
}

impl DrmLeaseDevice {
    pub fn new(raw: wp_drm_lease_device_v1::WpDrmLeaseDeviceV1) -> Self {
        Self { raw }
    }
}

impl wayland_server::Dispatch<wp_drm_lease_device_v1::WpDrmLeaseDeviceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wp_drm_lease_device_v1::WpDrmLeaseDeviceV1,
        request: <wp_drm_lease_device_v1::WpDrmLeaseDeviceV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wp_drm_lease_device_v1::Request::CreateLeaseRequest { id } => {
                let request = data_init.init(id, *data);
                todo!();
            }
            wp_drm_lease_device_v1::Request::Release => {
                state.destroy_object(resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_backend::server::ClientId,
        resource: &wp_drm_lease_device_v1::WpDrmLeaseDeviceV1,
        data: &Entity,
    ) {
        state.despawn_object_component::<DrmLeaseDevice>(*data, resource);
    }
}

impl wayland_server::GlobalDispatch<wp_drm_lease_device_v1::WpDrmLeaseDeviceV1, Entity> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wp_drm_lease_device_v1::WpDrmLeaseDeviceV1>,
        global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, DrmLeaseDevice::new);
    }
}

pub struct DrmLeasePlugin;
impl Plugin for DrmLeasePlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wp_drm_lease_device_v1::WpDrmLeaseDeviceV1, 1>(app);
    }
}
