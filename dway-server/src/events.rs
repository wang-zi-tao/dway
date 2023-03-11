use bevy::input::{keyboard::KeyboardInput, mouse::MouseButtonInput};
use dway_protocol::window::WindowState;
use smithay::{
    reexports::{wayland_server::protocol::wl_surface::WlSurface, x11rb::protocol::xproto::Window},
    utils::{Logical, Physical, Point, Rectangle},
    wayland::shell::xdg::{Configure, PopupSurface, PositionerState, ToplevelSurface},
    xwayland::{xwm::Reorder, X11Surface},
};

use crate::components::SurfaceId;

pub struct CreateWindow(pub SurfaceId);
pub struct DestroyWindow(pub SurfaceId);
pub struct CreateTopLevelEvent(pub ToplevelSurface);
pub struct ConfigureWindowNotify(pub SurfaceId, pub Configure);
pub struct CreatePopup(pub PopupSurface, pub PositionerState);
pub struct DestroyPopup(pub SurfaceId);
pub struct DestroyWlSurface(pub SurfaceId);
pub struct CreateX11WindowEvent {
    pub window: X11Surface,
    pub is_override_redirect: bool,
}
pub struct MapX11Window(pub SurfaceId);
pub struct UnmapX11Window(pub SurfaceId);
pub struct MapOverrideX11Window(pub SurfaceId);
pub struct X11WindowSetSurfaceEvent(pub SurfaceId);
pub struct ConfigureX11WindowRequest {
    pub window: SurfaceId,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub reorder: Option<Reorder>,
}
pub struct ConfigureX11WindowNotify {
    pub window: SurfaceId,
    pub geometry: Rectangle<i32, Logical>,
    pub above: Option<Window>,
}
pub struct DestroyX11WindowEvent(pub SurfaceId);
pub struct WindowSetGeometryEvent(pub SurfaceId, pub Rectangle<i32, Physical>);

pub struct CommitSurface(pub SurfaceId);
pub struct ShowWindowMenu(pub SurfaceId);
pub struct MoveRequest(pub SurfaceId);
pub struct ResizeRequest {
    pub surface: SurfaceId,
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}
pub struct SetState {
    pub surface: SurfaceId,
    pub state: WindowState,
    pub unset: bool,
}

pub struct GrabPopup(pub SurfaceId);
pub struct UpdatePopupPosition {
    pub surface_id: SurfaceId,
    pub positioner: PositionerState,
    pub token: u32,
}

pub struct CloseWindowRequest(pub SurfaceId);

pub struct MouseMoveOnWindow(pub SurfaceId, pub Point<i32, Logical>);
pub struct MouseButtonOnWindow(pub SurfaceId, pub Point<i32, Logical>, pub MouseButtonInput);
pub struct KeyboardInputOnWindw(pub SurfaceId, pub KeyboardInput);

pub struct NewDecoration(pub SurfaceId);
pub struct UnsetMode(pub SurfaceId);
