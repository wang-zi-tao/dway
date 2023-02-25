use bevy_ecs::prelude::*;
use failure::Fallible;
use smithay::{
    desktop::Window, reexports::wayland_server::Resource, wayland::shell::xdg::{ToplevelSurface, XdgShellHandler}, delegate_xdg_activation, delegate_xdg_shell,
};

use crate::{
    components::{WaylandWindow, WindowId, WindowIndex, WindowMark, WlSurfaceWrapper, UUID},
    // wayland::{
    //     DWayState,
    // },
    DWayBackend, events::{CreateTopLevelEvent, DestroyTopLevelEvent}, DWay,
};


#[derive(Bundle)]
pub struct WaylandWindowBundle {
    pub mark: WindowMark,
    pub window: WaylandWindow,
    pub uuid: UUID,
    pub id: WindowId,
}

pub fn create_top_level(
    mut create_top_level_event: EventReader<CreateTopLevelEvent>,
    mut window_index: Mut<WindowIndex>,
    mut commands: Commands,
) {
    for e in create_top_level_event.iter() {
        let top_level = &e.0;
        let wl_surface = top_level.wl_surface();
        let id = WindowId::from(wl_surface);
        let uuid = UUID::new();
        let entity = commands
            .spawn(WaylandWindowBundle {
                mark: WindowMark,
                window: WaylandWindow(Window::new(top_level.clone())),
                uuid,
                id: id.clone(),
            })
            .id();
        window_index.0.insert(id, entity);
    }
}
pub fn destroy_top_level(
    mut create_top_level_event: EventReader<DestroyTopLevelEvent>,
    mut window_index: Mut<WindowIndex>,
    mut commands: Commands,
) {
    for e in create_top_level_event.iter() {
        let id = WindowId::from(e.0.wl_surface());
        if let Some(entity) = window_index.0.get(&id) {
            commands.entity(*entity).despawn();
        }
        window_index.0.remove(&id);
    }
}

delegate_xdg_shell!(DWay);
impl XdgShellHandler for DWay{
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        todo!()
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        todo!()
    }

    fn new_popup(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface, positioner: smithay::wayland::shell::xdg::PositionerState) {
        todo!()
    }

    fn grab(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface, seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat, serial: smithay::utils::Serial) {
        todo!()
    }
}
