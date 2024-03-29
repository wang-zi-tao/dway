use super::seat::WlSeat;
use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::time,
    prelude::*,
    state::EntityFactory,
    util::serial::next_serial,
    wl::{
        cursor::{Cursor, PointerHasSurface},
        surface::WlSurface,
    },
};
use bevy::{input::mouse::MouseButtonInput, math::DVec2};

#[derive(Component, Reflect)]
pub struct WlPointer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_pointer::WlPointer,
    #[reflect(ignore)]
    pub focus: Option<wl_surface::WlSurface>,
}
#[derive(Bundle)]
pub struct WlPointerBundle {
    resource: WlPointer,
    pos: Geometry,
    global_pos: GlobalGeometry,
}

impl WlPointerBundle {
    pub fn new(resource: WlPointer) -> Self {
        Self {
            resource,
            pos: Default::default(),
            global_pos: Default::default(),
        }
    }
}

impl WlPointer {
    pub fn new(raw: wl_pointer::WlPointer) -> Self {
        Self { raw, focus: None }
    }
    pub fn frame(&self) {
        if self.raw.version() >= 5 {
            self.raw.frame();
        }
    }
    pub fn set_focus(
        &mut self,
        seat: &mut WlSeat,
        surface: &WlSurface,
        position: DVec2,
        serial: u32,
    ) -> bool {
        if !seat.enabled {
            return false;
        }
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                let focus = focus.clone();
                if !seat.can_focus_on(surface) {
                    return false;
                }
                if focus.is_alive() {
                    self.raw.leave(serial, &focus);
                    debug!(
                        serial,
                        "{} leave {} at {}",
                        self.raw.id(),
                        focus.id(),
                        position
                    );
                }
                debug!(
                    serial,
                    "{} enter {} at {}",
                    self.raw.id(),
                    surface.raw.id(),
                    position
                );
                self.raw.enter(serial, &surface.raw, position.x, position.y);
                self.focus = Some(surface.raw.clone());
            }
        } else {
            if !seat.can_focus_on(surface) {
                return false;
            }
            debug!(
                serial,
                "{} enter {} at {}",
                self.raw.id(),
                surface.raw.id(),
                position
            );
            self.raw.enter(serial, &surface.raw, position.x, position.y);
            self.focus = Some(surface.raw.clone());
        }
        true
    }
    pub fn button(
        &mut self,
        seat: &mut WlSeat,
        input: &MouseButtonInput,
        surface: &WlSurface,
        pos: DVec2,
    ) {
        let serial = next_serial();
        if !self.set_focus(seat, surface, pos, serial) {
            debug!(serial,surface=%WlResource::id( &surface.raw ),"cannot set focus");
            return;
        }
        debug!(serial,surface=%WlResource::id( &surface.raw ),"mouse button: {:?} at {:?}", input, pos);
        self.raw.button(
            next_serial(),
            time(),
            match input.button {
                MouseButton::Left => 0x110,
                MouseButton::Right => 0x111,
                MouseButton::Middle => 0x112,
                MouseButton::Forward => 0x115,
                MouseButton::Back => 0x116,
                MouseButton::Other(o) => o as u32,
            },
            match input.state {
                bevy::input::ButtonState::Pressed => wl_pointer::ButtonState::Pressed,
                bevy::input::ButtonState::Released => wl_pointer::ButtonState::Released,
            },
        );
        self.frame();
    }
    pub fn move_cursor(&mut self, seat: &mut WlSeat, surface: &WlSurface, delta: Vec2) {
        let serial = next_serial();
        if !self.set_focus(seat, surface, delta.as_dvec2(), serial) {
            debug!(serial,surface=%WlResource::id( &surface.raw ),"cannot set focus");
            return;
        }
        trace!("mouse move: {}", delta);
        self.raw.motion(time(), delta.x as f64, delta.y as f64);
        self.frame();
    }
    pub fn leave(&mut self) {
        let serial = next_serial();
        if let Some(focus) = &self.focus {
            if focus.is_alive() {
                self.raw.leave(serial, focus);
                debug!(serial, "{} leave {}", self.raw.id(), focus.id());
            }
            self.focus = None;
        }
    }
    pub fn asix(&mut self, seat: &mut WlSeat, value: DVec2, surface: &WlSurface, pos: DVec2) {
        let serial = next_serial();
        if !self.set_focus(seat, surface, pos, serial) {
            debug!(serial,surface=%WlResource::id( &surface.raw ),"cannot set focus");
            return;
        }
        trace!("axis: {}", pos);
        if value.x != 0.0 {
            self.raw
                .axis(time(), wl_pointer::Axis::HorizontalScroll, value.x);
        }
        if value.y != 0.0 {
            self.raw
                .axis(time(), wl_pointer::Axis::VerticalScroll, value.y);
        }
        self.frame();
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
        _client: &wayland_server::Client,
        resource: &wayland_server::protocol::wl_pointer::WlPointer,
        request: <wayland_server::protocol::wl_pointer::WlPointer as WlResource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_pointer::Request::SetCursor {
                serial,
                surface,
                hotspot_x,
                hotspot_y,
            } => {
                if let Some(surface) = surface {
                    state.insert(
                        DWay::get_entity(&surface),
                        (
                            Geometry::new(crate::util::rect::IRect::new(
                                -hotspot_x, -hotspot_y, 0, 0,
                            )),
                            GlobalGeometry::default(),
                            Cursor {
                                serial: Some(serial),
                            },
                        )
                            .with_parent(*data)
                            .connect_from::<PointerHasSurface>(*data),
                    );
                } else {
                    state.disconnect_all::<PointerHasSurface>(*data);
                }
            }
            wl_pointer::Request::Release => state.destroy_object(resource),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_pointer::WlPointer,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
