use smithay::{wayland::shell::xdg::ToplevelSurface, xwayland::X11Surface};

use crate::components::WindowId;



pub struct CreateWindow(pub WindowId);
pub struct DestroyWindow(pub WindowId);
pub struct CreateTopLevelEvent(pub ToplevelSurface);
pub struct DestroyTopLevelEvent(pub ToplevelSurface);
pub struct CreateX11WindowEvent(pub X11Surface);
pub struct DestroyX11WindowEvent(pub X11Surface);
