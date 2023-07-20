use std::sync::Arc;

use bevy_relationship::{relationship, AppExt};
use wayland_server::protocol::wl_seat::Capability;

use crate::{input::pointer::WlPointerBundle, prelude::*, state::create_global_system_config};

use super::{keyboard::WlKeyboard, pointer::WlPointer, touch::WlTouch};

#[derive(Component)]
pub struct WlSeat {
    raw: wl_seat::WlSeat,
}
relationship!(SeatHasPointer=>PointerList-<SeatOfPoint);
relationship!(SeatHasKeyboard=>KeyboardList-<SeatOfKeyboard);

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_seat::WlSeat: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<wayland_server::protocol::wl_seat::WlSeat, bevy::prelude::Entity, DWay>
    for SeatDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_seat::WlSeat,
        request: <wayland_server::protocol::wl_seat::WlSeat as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_seat::Request::GetPointer { id } => {
                let entity = state.spawn_child_object_bundle(*data, id, data_init, |o| {
                    WlPointerBundle::new(WlPointer::new(o))
                });
                state.connect::<SeatHasPointer>(*data, entity);
            }
            wl_seat::Request::GetKeyboard { id } => {
                let entity =
                    state.spawn_child_object_with_world(*data, id, data_init, |kbd, world| {
                        WlKeyboard::new(kbd, &world.resource()).unwrap()
                    });
                state.connect::<SeatHasKeyboard>(*data, entity);
            }
            wl_seat::Request::GetTouch { id } => {
                state.insert_object(*data, id, data_init, WlTouch::new);
            }
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
        state.despawn_object(*data, resource);
    }
}
impl wayland_server::GlobalDispatch<wayland_server::protocol::wl_seat::WlSeat, Entity> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wayland_server::protocol::wl_seat::WlSeat>,
        global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            o.capabilities(Capability::all());
            WlSeat { raw: o }
        });
    }
}

pub struct WlSeatPlugin;
impl Plugin for WlSeatPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(create_global_system_config::<wl_seat::WlSeat, 7>());
        app.register_relation::<SeatHasPointer>();
        app.add_plugin(super::keyboard::WlKeyboardPlugin);
    }
}
