use std::{sync::Arc, time::SystemTime};

use bevy::input::keyboard::KeyboardInput;

use crate::{prelude::*, util::serial::next_serial};

#[derive(Component)]
pub struct WlKeyboard {
    pub raw: wl_keyboard::WlKeyboard,
}

impl WlKeyboard {
    pub fn new(raw: wl_keyboard::WlKeyboard) -> Self {
        Self { raw }
    }
    pub fn key(&self, input: KeyboardInput) {
        self.raw.key(
            next_serial(),
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            input.scan_code,
            match input.state {
                bevy::input::ButtonState::Pressed => wl_keyboard::KeyState::Pressed,
                bevy::input::ButtonState::Released => wl_keyboard::KeyState::Released,
            },
        );
    }
}

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_keyboard::WlKeyboard: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<
        wayland_server::protocol::wl_keyboard::WlKeyboard,
        bevy::prelude::Entity,
        DWay,
    > for SeatDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_keyboard::WlKeyboard,
        request: <wayland_server::protocol::wl_keyboard::WlKeyboard as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_keyboard::Request::Release => todo!(),
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
