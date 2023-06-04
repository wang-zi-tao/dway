use std::{sync::Arc, time::SystemTime};

use bevy::input::mouse::MouseButtonInput;

use crate::{prelude::*, util::serial::next_serial, wl::surface::WlSurface};

#[derive(Component)]
pub struct WlPointer {
    pub raw: wl_pointer::WlPointer,
}

impl WlPointer {
    pub fn new(raw: wl_pointer::WlPointer) -> Self {
        Self { raw }
    }
    pub fn button(&self, input: MouseButtonInput) {
        self.raw.button(
            next_serial(),
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            match input.button {
                MouseButton::Left => 0x110,
                MouseButton::Right => 0x111,
                MouseButton::Middle => 0x112,
                MouseButton::Other(o) => o as u32,
            },
            match input.state {
                bevy::input::ButtonState::Pressed => wl_pointer::ButtonState::Pressed,
                bevy::input::ButtonState::Released => wl_pointer::ButtonState::Released,
            },
        );
    }
    pub fn move_cursor(&self, delta: Vec2) {
        self.raw.motion(
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            delta.x as f64,
            delta.y as f64,
        );
    }
    pub fn vertical_asix(&self, value: f64) {
        self.raw.axis(
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            wl_pointer::Axis::VerticalScroll,
            value,
        );
    }
    pub fn horizontal_asix(&self, value: f64) {
        self.raw.axis(
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            wl_pointer::Axis::HorizontalScroll,
            value,
        );
    }
    pub fn enter(&self, surface: &WlSurface, position: Vec2) {
        self.raw.enter(
            next_serial(),
            &surface.raw,
            position.x as f64,
            position.y as f64,
        );
    }
    pub fn leave(&self, surface: &WlSurface) {
        self.raw.leave(
            next_serial(),
            &surface.raw,
        );
    }
}

#[derive(Resource)]
pub struct SeatDelegate(pub GlobalId);

delegate_dispatch!(DWay: [wl_pointer::WlPointer: Entity] => SeatDelegate);

impl
    wayland_server::Dispatch<
        wayland_server::protocol::wl_pointer::WlPointer,
        bevy::prelude::Entity,
        DWay,
    > for SeatDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_pointer::WlPointer,
        request: <wayland_server::protocol::wl_pointer::WlPointer as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_pointer::Request::Release => todo!(),
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
