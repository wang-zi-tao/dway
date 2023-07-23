use std::{sync::Arc, time::SystemTime};

use bevy::{input::mouse::MouseButtonInput, math::DVec2};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    state::EntityFactory,
    util::serial::next_serial,
    wl::{
        cursor::{Cursor, PointerHasSurface},
        surface::WlSurface,
    },
};

use super::grab::PointerGrab;

#[derive(Component)]
pub struct WlPointer {
    pub raw: wl_pointer::WlPointer,
    pub focus: Option<wl_surface::WlSurface>,
    pub grab_by: Option<wl_surface::WlSurface>,
}
#[derive(Bundle)]
pub struct WlPointerBundle {
    resource: WlPointer,
    pos: Geometry,
    global_pos: GlobalGeometry,
    grab: PointerGrab,
}

impl WlPointerBundle {
    pub fn new(resource: WlPointer) -> Self {
        Self {
            resource,
            pos: Default::default(),
            global_pos: Default::default(),
            grab: Default::default(),
        }
    }
}

impl WlPointer {
    pub fn new(raw: wl_pointer::WlPointer) -> Self {
        Self {
            raw,
            focus: None,
            grab_by: None,
        }
    }
    pub fn can_focus_on(&self, surface: &WlSurface) -> bool {
        self.grab_by
            .as_ref()
            .map(|s| s == &surface.raw)
            .unwrap_or(true)
    }
    pub fn set_focus(&mut self, surface: &WlSurface, position: DVec2) {
        if !self.can_focus_on(surface) {
            return;
        }
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                if focus.is_alive() {
                    self.raw.leave(next_serial(), &focus);
                    debug!("{} leave {}", self.raw.id(), focus.id());
                }
                debug!("{} enter {}", self.raw.id(), surface.raw.id());
                self.raw
                    .enter(next_serial(), &surface.raw, position.x, position.y);
                self.focus = Some(surface.raw.clone());
            }
        } else {
            debug!("{} enter {}", self.raw.id(), surface.raw.id());
            self.raw
                .enter(next_serial(), &surface.raw, position.x, position.y);
            self.focus = Some(surface.raw.clone());
        }
    }
    pub fn button(&mut self, input: &MouseButtonInput, surface: &WlSurface, pos: DVec2) {
        if !self.can_focus_on(surface) {
            return;
        }
        self.set_focus(surface, pos);
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
    pub fn move_cursor(&mut self, surface: &WlSurface, delta: Vec2) {
        if !self.can_focus_on(surface) {
            return;
        }
        self.set_focus(surface, delta.as_dvec2());
        self.raw.motion(
            SystemTime::now().elapsed().unwrap().as_millis() as u32,
            delta.x as f64,
            delta.y as f64,
        );
        self.raw.frame();
    }
    pub fn leave(&mut self) {
        if self.grab_by.is_some() {
            return;
        }
        if let Some(focus) = &self.focus {
            if focus.is_alive() {
                self.raw.leave(next_serial(), focus);
                debug!("{} leave {}", self.raw.id(), focus.id());
            }
            self.focus = None;
        }
    }
    pub fn asix(&mut self, value: DVec2, surface: &WlSurface, pos: DVec2) {
        if !self.can_focus_on(surface) {
            return;
        }
        self.set_focus(surface, pos);
        if value.x != 0.0 {
            self.raw.axis(
                SystemTime::now().elapsed().unwrap().as_millis() as u32,
                wl_pointer::Axis::HorizontalScroll,
                value.x,
            );
        }
        if value.y != 0.0 {
            self.raw.axis(
                SystemTime::now().elapsed().unwrap().as_millis() as u32,
                wl_pointer::Axis::VerticalScroll,
                value.y,
            );
        }
        self.raw.frame();
    }
    pub fn unset_grab(&mut self) {
        self.grab_by = None;
    }
    pub fn grab_raw(&mut self, surface: &wl_surface::WlSurface) {
        self.grab_by = Some(surface.clone());
    }
    pub fn grab(&mut self, surface: &WlSurface) {
        self.grab_by = Some(surface.raw.clone());
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
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
