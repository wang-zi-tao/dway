use crate::prelude::*;
use super::grab::Grab;

#[derive(Component)]
pub struct WlTouch {
    pub raw: wl_touch::WlTouch,
}

impl WlTouch {
    pub fn new(raw: wl_touch::WlTouch) -> Self {
        Self { raw }
    }
}

#[derive(Bundle)]
pub struct WlTouchBundle {
    resource: WlTouch,
    grab: Grab,
}

impl WlTouchBundle {
    pub fn new(resource: WlTouch) -> Self {
        Self {
            resource,
            grab: Default::default(),
        }
    }
}

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_touch::WlTouch: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<
        wayland_server::protocol::wl_touch::WlTouch,
        bevy::prelude::Entity,
        DWay,
    > for SeatDelegate
{
    fn request(
        _state: &mut DWay,
        _client: &wayland_server::Client,
        _resource: &wayland_server::protocol::wl_touch::WlTouch,
        request: <wayland_server::protocol::wl_touch::WlTouch as WlResource>::Request,
        _data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_touch::Request::Release => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_touch::WlTouch,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
