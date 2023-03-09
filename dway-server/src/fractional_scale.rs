use smithay::{delegate_fractional_scale, wayland::fractional_scale::FractionScaleHandler};

use crate::DWay;
use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_popup::XdgPopup,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    xwayland::X11Surface,
};

impl FractionScaleHandler for DWay {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        todo!()
    }
}
delegate_fractional_scale!(DWay);
