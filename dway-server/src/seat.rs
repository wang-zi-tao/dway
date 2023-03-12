use crate::DWay;
use bevy::prelude::Entity;
use smithay::{
    delegate_seat,
    input::{keyboard::KeyboardTarget, pointer::PointerTarget, SeatHandler},
    utils::IsAlive,
    wayland::{seat::WaylandFocus, shell::xdg::PopupSurface},
    xwayland::X11Surface,
};
use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::protocol::wl_surface::WlSurface,
        wayland_server::Resource,
    },
    wayland::shell::xdg::{ToplevelStateSet, ToplevelSurface, XdgShellHandler},
};

#[derive(PartialEq, Clone)]
pub enum KeyboardFocus {}
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
pub enum PointerFocus {
    WaylandWindow(Window),
    X11Window(X11Surface),
    Popup(PopupSurface),
}
impl IsAlive for PointerFocus {
    fn alive(&self) -> bool {
        match self {
            PointerFocus::WaylandWindow(w) => IsAlive::alive(w),
            PointerFocus::X11Window(w) => IsAlive::alive(w),
            PointerFocus::Popup(w) => IsAlive::alive(w.wl_surface()),
        }
    }
}
impl WaylandFocus for PointerFocus {
    fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            PointerFocus::WaylandWindow(w) => w.wl_surface(),
            PointerFocus::X11Window(w) => w.wl_surface(),
            PointerFocus::Popup(w) => Some(w.wl_surface().clone()),
        }
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
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::enter(w, seat, data, event),
            PointerFocus::X11Window(w) => PointerTarget::enter(w, seat, data, event),
            PointerFocus::Popup(w) => PointerTarget::enter(w.wl_surface(), seat, data, event),
        }
    }

    fn motion(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::MotionEvent,
    ) {
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::motion(w, seat, data, event),
            PointerFocus::X11Window(w) => PointerTarget::motion(w, seat, data, event),
            PointerFocus::Popup(w) => PointerTarget::motion(w.wl_surface(), seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::RelativeMotionEvent,
    ) {
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::relative_motion(w, seat, data, event),
            PointerFocus::X11Window(w) => PointerTarget::relative_motion(w, seat, data, event),
            PointerFocus::Popup(w) => {
                PointerTarget::relative_motion(w.wl_surface(), seat, data, event)
            }
        }
    }

    fn button(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        event: &smithay::input::pointer::ButtonEvent,
    ) {
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::button(w, seat, data, event),
            PointerFocus::X11Window(w) => PointerTarget::button(w, seat, data, event),
            PointerFocus::Popup(w) => PointerTarget::button(w.wl_surface(), seat, data, event),
        }
    }

    fn axis(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        frame: smithay::input::pointer::AxisFrame,
    ) {
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::axis(w, seat, data, frame),
            PointerFocus::X11Window(w) => PointerTarget::axis(w, seat, data, frame),
            PointerFocus::Popup(w) => PointerTarget::axis(w.wl_surface(), seat, data, frame),
        }
    }

    fn leave(
        &self,
        seat: &smithay::input::Seat<DWay>,
        data: &mut DWay,
        serial: smithay::utils::Serial,
        time: u32,
    ) {
        match self {
            PointerFocus::WaylandWindow(w) => PointerTarget::leave(w, seat, data, serial, time),
            PointerFocus::X11Window(w) => PointerTarget::leave(w, seat, data, serial, time),
            PointerFocus::Popup(w) => {
                PointerTarget::leave(w.wl_surface(), seat, data, serial, time)
            }
        }
    }
}

impl SeatHandler for DWay {
    type KeyboardFocus = WlSurface;

    type PointerFocus = WlSurface;

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
