use crate::{prelude::*, state::add_global_dispatch};

#[derive(Component)]
pub struct IdleInhibitManager {
    pub raw: zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
}

impl IdleInhibitManager {
    pub fn new(raw: zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1) -> Self {
        Self { raw }
    }
}

impl wayland_server::Dispatch<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
        request: <zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_idle_inhibit_manager_v1::Request::Destroy => {
                state.destroy_object(resource);
            }
            zwp_idle_inhibit_manager_v1::Request::CreateInhibitor { id, surface } => todo!(),
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_backend::server::ClientId,
        resource: &zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1,
        data: &Entity,
    ) {
        state.despawn_object_component::<IdleInhibitManager>(*data, resource);
    }
}

impl wayland_server::GlobalDispatch<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, Entity>
    for DWay
{
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1>,
        global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, IdleInhibitManager::new);
    }
}

pub struct IdlePlugin;
impl Plugin for IdlePlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<zwp_idle_inhibit_manager_v1::ZwpIdleInhibitManagerV1, 1>(app);
    }
}
