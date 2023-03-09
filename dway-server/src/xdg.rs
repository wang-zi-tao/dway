use smithay::{
    delegate_xdg_activation, delegate_xdg_decoration,
    reexports::wayland_protocols,
    wayland::{shell::xdg::decoration::XdgDecorationHandler, xdg_activation::XdgActivationHandler},
};

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_popup::XdgPopup,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    xwayland::X11Surface,
};
use wayland_protocols::xdg::decoration::zv1::server::{
    zxdg_decoration_manager_v1,
    zxdg_toplevel_decoration_v1::{self, Mode},
};

use crate::{
    events::{NewDecoration, UnsetMode},
    DWay,
};

impl XdgActivationHandler for DWay {
    fn activation_state(&mut self) -> &mut smithay::wayland::xdg_activation::XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        token: smithay::wayland::xdg_activation::XdgActivationToken,
        token_data: smithay::wayland::xdg_activation::XdgActivationTokenData,
        surface: WlSurface,
    ) {
        todo!()
    }

    fn destroy_activation(
        &mut self,
        token: smithay::wayland::xdg_activation::XdgActivationToken,
        token_data: smithay::wayland::xdg_activation::XdgActivationTokenData,
        surface: WlSurface,
    ) {
        todo!()
    }
}
delegate_xdg_activation!(DWay);
impl XdgDecorationHandler for DWay {
    fn new_decoration(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
        self.send_ecs_event(NewDecoration(toplevel.into()));
    }

    fn request_mode(
        &mut self,
        toplevel: smithay::wayland::shell::xdg::ToplevelSurface,
        mode: Mode,
    ) {
        todo!()
    }

    fn unset_mode(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        self.send_ecs_event(UnsetMode(toplevel.into()));
    }
}
delegate_xdg_decoration!(DWay);
