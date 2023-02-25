use bevy_ecs::prelude::*;
use failure::Fallible;
use smithay::{
    desktop::Window, reexports::wayland_server::Resource, wayland::shell::xdg::ToplevelSurface,
};

use crate::{
    components::{WaylandWindow, WindowId, WindowIndex, WindowMark, WlSurfaceWrapper, UUID, X11Window},
    // wayland::{
    //     DWayState,
    // },
    DWayBackend, events::{CreateTopLevelEvent, DestroyTopLevelEvent, CreateX11WindowEvent, DestroyX11WindowEvent},
};


#[derive(Bundle)]
pub struct X11WindowBundle {
    pub mark: WindowMark,
    pub window: X11Window,
    pub uuid: UUID,
    pub id: WindowId,
}

pub fn create_x11_surface(
    mut events: EventReader<CreateX11WindowEvent>,
    mut window_index: Mut<WindowIndex>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let x11_surface = &e.0;
        // let wl_surface = x11_surface.wl_surface();
        let id = WindowId::from(x11_surface);
        let uuid = UUID::new();
        let entity = commands
            .spawn(X11WindowBundle {
                mark: WindowMark,
                window: X11Window(x11_surface.clone()),
                uuid,
                id: id.clone(),
            })
            .id();
        window_index.0.insert(id, entity);
    }
}
pub fn destroy_x11_surface(
    mut events: EventReader<DestroyX11WindowEvent>,
    mut window_index: Mut<WindowIndex>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let id = WindowId::from(&e.0);
        if let Some(entity) = window_index.0.get(&id) {
            commands.entity(*entity).despawn();
        }
        window_index.0.remove(&id);
    }
}
