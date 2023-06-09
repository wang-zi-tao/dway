use std::{sync::Arc, time::SystemTime};

use bevy::input::keyboard::KeyboardInput;

use crate::{prelude::*, util::serial::next_serial, wl::surface::WlSurface};

#[derive(Component)]
pub struct WlKeyboard {
    pub raw: wl_keyboard::WlKeyboard,
    pub focus: Option<wl_surface::WlSurface>,
}

impl WlKeyboard {
    pub fn new(kbd: wl_keyboard::WlKeyboard) -> Self {
        Self {
            raw: kbd,
            focus: None,
        }
    }
    pub fn set_focus(&mut self, surface: &WlSurface) {
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                self.raw.leave(next_serial(), &focus);
                trace!("{} leave {}", self.raw.id(), focus.id());
                self.raw.enter(next_serial(), &surface.raw, Vec::new());
                trace!("{} enter {}", self.raw.id(), surface.raw.id());
                self.focus = Some(surface.raw.clone());
            }
        } else {
            self.raw.enter(next_serial(), &surface.raw, Vec::new());
            self.focus = Some(surface.raw.clone());
            trace!("{} enter {}", self.raw.id(), surface.raw.id());
        }
    }
    pub fn key(&self, surface: &WlSurface, input: &KeyboardInput) {
        trace!(surface=?surface.raw.id(),"key evnet : {input:?}");
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
