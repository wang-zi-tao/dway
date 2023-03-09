use crate::DWay;
use bevy::prelude::Entity;
use smithay::{
    delegate_seat,
    input::{keyboard::KeyboardTarget, pointer::PointerTarget, SeatHandler},
    utils::IsAlive,
    wayland::seat::WaylandFocus,
};
use smithay::{
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::protocol::wl_surface::WlSurface,
        wayland_server::Resource,
    },
    wayland::shell::xdg::{ToplevelStateSet, ToplevelSurface, XdgShellHandler},
};

#[derive(PartialEq, Clone)]
pub struct KeyboardFocus {}
impl WaylandFocus for KeyboardFocus {
    fn wl_surface(&self) -> Option<WlSurface> {
        todo!()
    }

    fn same_client_as(
        &self,
        object_id: &smithay::reexports::wayland_server::backend::ObjectId,
    ) -> bool {
        self.wl_surface()
            .map(|s| s.id().same_client_as(object_id))
            .unwrap_or(false)
    }
}
impl IsAlive for KeyboardFocus {
    fn alive(&self) -> bool {
        todo!()
    }
}
impl KeyboardTarget<DWay> for KeyboardFocus {
    fn enter(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        keys: Vec<smithay::input::keyboard::KeysymHandle<'_>>,
        serial: smithay::utils::Serial,
    ) {
        todo!()
    }

    fn leave(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        serial: smithay::utils::Serial,
    ) {
        todo!()
    }

    fn key(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        key: smithay::input::keyboard::KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        todo!()
    }

    fn modifiers(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: smithay::utils::Serial,
    ) {
        todo!()
    }
}
#[derive(PartialEq, Clone)]
pub struct PointerFocus {}
impl IsAlive for PointerFocus {
    fn alive(&self) -> bool {
        todo!()
    }
}
impl WaylandFocus for PointerFocus {
    fn wl_surface(&self) -> Option<WlSurface> {
        todo!()
    }

    fn same_client_as(
        &self,
        object_id: &smithay::reexports::wayland_server::backend::ObjectId,
    ) -> bool {
        self.wl_surface()
            .map(|s| s.id().same_client_as(object_id))
            .unwrap_or(false)
    }
}
impl PointerTarget<DWay> for PointerFocus {
    fn enter(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        todo!()
    }

    fn motion(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        todo!()
    }

    fn relative_motion(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        todo!()
    }

    fn button(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        todo!()
    }

    fn axis(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        todo!()
    }

    fn leave(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        todo!()
    }
}

impl SeatHandler for DWay {
    type KeyboardFocus = KeyboardFocus;

    type PointerFocus = PointerFocus;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _focused: Option<&Self::KeyboardFocus>,
    ) {
    }

    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
}
delegate_seat!(DWay);
