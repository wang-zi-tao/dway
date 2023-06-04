use std::sync::Arc;

use crate::prelude::*;

use super::{pointer::WlPointer, keyboard::WlKeyboard, touch::WlTouch};

#[derive(Component)]
pub struct WlSeat{
    raw:wl_seat::WlSeat,
}

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_seat::WlSeat: Entity] => SeatDelegate);

impl wayland_server::Dispatch<wayland_server::protocol::wl_seat::WlSeat, bevy::prelude::Entity, DWay> for SeatDelegate{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_seat::WlSeat,
        request: <wayland_server::protocol::wl_seat::WlSeat as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request{
            wl_seat::Request::GetPointer { id } => {
                state.insert_object(*data, id, data_init, WlPointer::new);
            },
            wl_seat::Request::GetKeyboard { id } => {
                state.insert_object(*data, id, data_init, WlKeyboard::new);
            },
            wl_seat::Request::GetTouch { id } => {
                state.insert_object(*data, id, data_init, WlTouch::new);
            },
            wl_seat::Request::Release => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data,resource);
    }
}
impl wayland_server::GlobalDispatch<wayland_server::protocol::wl_seat::WlSeat, ()> for DWay{
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wayland_server::protocol::wl_seat::WlSeat>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.init_object(resource, data_init, |o| WlSeat { raw: o });
    }
}

pub struct WlSeatPlugin(pub Arc<DisplayHandle>);
impl Plugin for WlSeatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SeatDelegate ( self
                .0
                .create_global::<DWay, wl_seat::WlSeat, ()>(7, ()), ));
    }
}
