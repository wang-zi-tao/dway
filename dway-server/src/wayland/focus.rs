pub use smithay::{
    backend::input::KeyState,
    desktop::{LayerSurface, PopupKind},
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{AxisFrame, ButtonEvent, MotionEvent, PointerTarget},
        Seat,
    },
    reexports::wayland_server::{backend::ObjectId, protocol::wl_surface::WlSurface, Resource},
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
};

use super::{shell::WindowElement, DWayState, };


#[derive(Debug,Clone,PartialEq)]
pub enum FocusTarget {
    Window(WindowElement),
    LayerSurface(LayerSurface),
    Popup(PopupKind),
}
impl PointerTarget<DWayState> for FocusTarget {
    fn enter(&self, seat: &Seat<DWayState>, data: &mut DWayState, event: &MotionEvent) {
        match self {
            FocusTarget::Window(w) => PointerTarget::enter(w, seat, data, event),
            FocusTarget::LayerSurface(l) => PointerTarget::enter(l, seat, data, event),
            FocusTarget::Popup(p) => PointerTarget::enter(p.wl_surface(), seat, data, event),
        }
    }

    fn motion(&self, seat: &Seat<DWayState>, data: &mut DWayState, event: &MotionEvent) {
        match self{
            FocusTarget::Window(w) => w.motion(seat, data, event),
            FocusTarget::LayerSurface(w) => w.motion(seat, data, event),
            FocusTarget::Popup(w) => w.wl_surface().motion(seat, data, event),
        }
    }

    fn button(&self, seat: &Seat<DWayState>, data: &mut DWayState, event: &ButtonEvent) {
        match self{
            FocusTarget::Window(w) => w.button(seat, data, event),
            FocusTarget::LayerSurface(w) => w.button(seat, data, event),
            FocusTarget::Popup(w) => w.wl_surface().button(seat, data, event),
        }
    }

    fn axis(&self, seat: &Seat<DWayState>, data: &mut DWayState, frame: AxisFrame) {
        match self{
            FocusTarget::Window(w) => w.axis(seat, data, frame),
            FocusTarget::LayerSurface(w) => w.axis(seat, data, frame),
            FocusTarget::Popup(w) => w.wl_surface().axis(seat, data, frame),
        }
    }

    fn leave(&self, seat: &Seat<DWayState>, data: &mut DWayState, serial: Serial, time: u32) {
        match self{
            FocusTarget::Window(w) => PointerTarget::leave(w, seat, data, serial, time),
            FocusTarget::LayerSurface(w) => PointerTarget::leave(w, seat, data, serial, time),
            FocusTarget::Popup(w) => PointerTarget::leave(w.wl_surface(), seat, data, serial, time),
        }
    }

    fn relative_motion(&self, seat: &Seat<DWayState>, data: &mut DWayState, event: &smithay::input::pointer::RelativeMotionEvent) {
        match self{
            FocusTarget::Window(w) => PointerTarget::relative_motion(w, seat, data, event),
            FocusTarget::LayerSurface(w) => PointerTarget::relative_motion(w, seat, data, event),
            FocusTarget::Popup(w) => PointerTarget::relative_motion(w.wl_surface(), seat, data, event),
        }
    }
}

impl KeyboardTarget<DWayState> for FocusTarget {
    fn enter(&self, seat: &Seat<DWayState>, data: &mut DWayState, keys: Vec<KeysymHandle<'_>>, serial: Serial) {
        match self {
            FocusTarget::Window(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
            FocusTarget::LayerSurface(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
            FocusTarget::Popup(w) => KeyboardTarget::enter(w.wl_surface(), seat, data, keys, serial),
        }
    }

    fn leave(&self, seat: &Seat<DWayState>, data: &mut DWayState, serial: Serial) {
        match self{
            FocusTarget::Window(w) => KeyboardTarget::leave(w, seat, data, serial),
            FocusTarget::LayerSurface(w) => KeyboardTarget::leave(w, seat, data, serial),
            FocusTarget::Popup(w) => KeyboardTarget::leave(w.wl_surface(), seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &Seat<DWayState>,
        data: &mut DWayState,
        key: KeysymHandle<'_>,
        state: KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self{
            FocusTarget::Window(w) => KeyboardTarget::key(w, seat, data, key, state, serial, time),
            FocusTarget::LayerSurface(w) => KeyboardTarget::key(w, seat, data, key, state, serial, time),
            FocusTarget::Popup(w) => KeyboardTarget::key(w.wl_surface(), seat, data, key, state, serial, time),
        }
    }

    fn modifiers(&self, seat: &Seat<DWayState>, data: &mut DWayState, modifiers: ModifiersState, serial: Serial) {
        match self{
            FocusTarget::Window(w) => KeyboardTarget::modifiers(w, seat, data, modifiers, serial),
            FocusTarget::LayerSurface(w) => KeyboardTarget::modifiers(w, seat, data, modifiers, serial),
            FocusTarget::Popup(w) => KeyboardTarget::modifiers(w.wl_surface(), seat, data, modifiers, serial),
        }
    }
}

impl WaylandFocus for FocusTarget {
    fn wl_surface(&self) -> Option<smithay::reexports::wayland_server::protocol::wl_surface::WlSurface> {
        match self{
            FocusTarget::Window(w) => w.wl_surface(),
            FocusTarget::LayerSurface(w) => Some(w.wl_surface().clone()),
            FocusTarget::Popup(w) => Some(w.wl_surface().clone())
        }
    }
}

impl From<WindowElement> for FocusTarget {
    fn from(value: WindowElement) -> Self {
        FocusTarget::Window(value)
    }
}
impl From<LayerSurface> for FocusTarget {
    fn from(value: LayerSurface) -> Self {
        Self::LayerSurface(value)
    }
}
impl From<PopupKind> for FocusTarget {
    fn from(value: PopupKind) -> Self {
        Self::Popup(value)
    }
}

impl IsAlive for FocusTarget {
    fn alive(&self) -> bool {
        match self{
            FocusTarget::Window(w) => w.alive(),
            FocusTarget::LayerSurface(w) => w.alive(),
            FocusTarget::Popup(w) => w.alive(),
        }
    }
}
