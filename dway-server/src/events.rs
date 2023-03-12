use bevy::input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};
use dway_protocol::window::WindowState;
use smithay::{
    reexports::{wayland_server::protocol::wl_surface::WlSurface, x11rb::protocol::xproto::Window},
    utils::{Logical, Physical, Point, Rectangle, Size},
    wayland::shell::xdg::{Configure, PopupSurface, PositionerState, ToplevelSurface},
    xwayland::{xwm::Reorder, X11Surface},
};

use crate::components::SurfaceId;

#[derive(Clone, Debug)]
pub struct CreateWindow(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct DestroyWindow(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct CreateTopLevelEvent(pub ToplevelSurface);
#[derive(Debug)]
pub struct ConfigureWindowNotify(pub SurfaceId, pub Configure);
#[derive(Clone, Debug)]
pub struct CreatePopup(pub PopupSurface, pub PositionerState);
#[derive(Clone, Debug)]
pub struct DestroyPopup(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct DestroyWlSurface(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct CreateX11WindowEvent {
    pub window: X11Surface,
    pub is_override_redirect: bool,
}
#[derive(Clone, Debug)]
pub struct MapX11Window(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct UnmapX11Window(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct MapOverrideX11Window(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct X11WindowSetSurfaceEvent(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct ConfigureX11WindowRequest {
    pub window: SurfaceId,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub reorder: Option<Reorder>,
}
#[derive(Clone, Debug)]
pub struct ConfigureX11WindowNotify {
    pub window: SurfaceId,
    pub geometry: Rectangle<i32, Logical>,
    pub above: Option<Window>,
}
#[derive(Clone, Debug)]
pub struct DestroyX11WindowEvent(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct WindowSetGeometryEvent(pub SurfaceId, pub Rectangle<i32, Physical>);

#[derive(Clone, Debug)]
pub struct CommitSurface {
    pub surface: SurfaceId,
    pub surface_size: Option<Size<i32,Logical> >,
}
#[derive(Clone, Debug)]
pub struct ShowWindowMenu(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct MoveRequest(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct ResizeRequest {
    pub surface: SurfaceId,
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}
#[derive(Clone, Debug)]
pub struct SetState {
    pub surface: SurfaceId,
    pub state: WindowState,
    pub unset: bool,
}

#[derive(Clone, Debug)]
pub struct GrabPopup(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct UpdatePopupPosition {
    pub surface_id: SurfaceId,
    pub positioner: PositionerState,
    pub token: u32,
}

#[derive(Clone, Debug)]
pub struct CloseWindowRequest(pub SurfaceId);

#[derive(Clone, Debug)]
pub struct MouseMoveOnWindow(pub SurfaceId, pub Point<i32, Logical>);
#[derive(Clone, Debug)]
pub struct MouseMotionOnWindow(pub SurfaceId, pub Point<i32, Logical>);
#[derive(Clone, Debug)]
pub struct MouseButtonOnWindow(pub SurfaceId, pub Point<i32, Logical>, pub MouseButtonInput);
#[derive(Clone, Debug)]
pub struct MouseWheelOnWindow(pub SurfaceId, pub Point<i32, Logical>, pub MouseWheel);
#[derive(Clone, Debug)]
pub struct KeyboardInputOnWindow(pub SurfaceId, pub KeyboardInput);

#[derive(Clone, Debug)]
pub struct NewDecoration(pub SurfaceId);
#[derive(Clone, Debug)]
pub struct UnsetMode(pub SurfaceId);
