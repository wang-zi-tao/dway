use std::{sync::Arc, time::SystemTime};

use bevy::{input::mouse::MouseButtonInput, math::DVec2};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    util::serial::next_serial,
    wl::{
        cursor::{Cursor, PointerHasSurface},
        surface::WlSurface,
    },
};

#[derive(Component)]
pub struct WlPointer {
    pub raw: wl_pointer::WlPointer,
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
    pub fn set_focus(&mut self, surface: &WlSurface, position: DVec2) {
        if let Some(focus) = &self.focus {
            if &surface.raw != focus {
                self.raw.leave(next_serial(), &focus);
                trace!("{} leave {}", self.raw.id(), focus.id());
                self.raw
                    .enter(next_serial(), &surface.raw, position.x, position.y);
                self.focus = Some(surface.raw.clone());
                trace!("{} enter {}", self.raw.id(), surface.raw.id());
            }
        } else {
            self.raw
                .enter(next_serial(), &surface.raw, position.x, position.y);
            self.focus = Some(surface.raw.clone());
            trace!("{} enter {}", self.raw.id(), surface.raw.id());
        }
    }
    pub fn button(&self, input: &MouseButtonInput) {
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
        self.set_focus(surface, delta.as_dvec2());
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
                    debug!("set cursor to {}", surface.id());
                    let entity = state.insert_child(
                        *data,
                        DWay::get_entity(&surface),
                        (
                            Geometry::new(crate::util::rect::IRect::new(
                                -hotspot_x, -hotspot_y, 0, 0,
                            )),
                            GlobalGeometry::default(),
                            Cursor {
                                serial: Some(serial),
                            },
                        ),
                    );
                    state.connect::<PointerHasSurface>(*data, entity);
                } else {
                    state.disconnect_all::<PointerHasSurface>(*data);
                }
            }
            wl_pointer::Request::Release => state.destroy_object::<WlPointer>(resource),
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
