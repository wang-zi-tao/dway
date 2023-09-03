mod display;
pub mod events;
use std::sync::Arc;

pub use display::*;
pub mod screen;
pub mod util;
pub mod window;
use dway_winit::FrameConditionSchedule;

use crate::{
    client::{self, Client, ClientData, ClientEvents},
    prelude::*,
    schedule::{DWayServerSet, DWayStartSet},
    state::{on_create_display_event, DWayDisplay, DWayWrapper, WaylandDisplayCreated},
};

relationship!(XDisplayHasWindow=>XWindowList-<XDisplayRef);

use self::{
    events::{dispatch_x11_events, flush_xwayland, x11_frame_condition},
    window::{x11_window_attach_wl_surface, MappedXWindow, XWindow, XWindowAttachSurface},
};

#[derive(Bundle)]
pub struct XWaylandBundle {
    pub display: XWaylandDisplayWrapper,
    pub client: Client,
}

pub fn launch_xwayland(
    display_query: Query<(&DWayWrapper, &DWayDisplay)>,
    mut events: EventReader<WaylandDisplayCreated>,
    client_events: Res<ClientEvents>,
    mut commands: Commands,
) {
    for WaylandDisplayCreated(entity, _) in events.iter() {
        if let Ok((dway_wrapper, display_wrapper)) = display_query.get(*entity) {
            let mut dway = dway_wrapper.0.lock().unwrap();
            let display = display_wrapper.0.lock().unwrap();
            if let Err(e) =
                XWaylandDisplay::spawn(&mut dway, &display, *entity, &mut commands, &client_events)
            {
                error!(error=%e,"failed to launch xwayland");
            };
        }
    }
}

pub struct DWayXWaylandReady {
    pub dway_entity: Entity,
}

impl DWayXWaylandReady {
    pub fn new(dway_entity: Entity) -> Self {
        Self { dway_entity }
    }
}
pub struct DWayXWaylandStoped {
    pub dway_entity: Entity,
}

impl DWayXWaylandStoped {
    pub fn new(dway_entity: Entity) -> Self {
        Self { dway_entity }
    }
}
relationship!(DWayHasXWayland=>XWaylandRef--DWayRef);

pub struct DWayXWaylandPlugin;
impl Plugin for DWayXWaylandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                launch_xwayland.run_if(on_event::<WaylandDisplayCreated>()),
                apply_system_buffers,
            )
                .chain()
                .in_set(DWayServerSet::Create)
                .after(on_create_display_event),
        );
        app.add_system(x11_frame_condition.in_schedule(FrameConditionSchedule));
        app.add_system(dispatch_x11_events.in_set(DWayServerSet::Dispatch));
        app.register_relation::<XDisplayHasWindow>();
        app.add_system(x11_window_attach_wl_surface.in_set(DWayServerSet::UpdateXWayland));
        // app.add_system(flush_xwayland.in_set(DWayServerSet::PostUpdate));
        app.register_type::<XWindow>();
        app.register_type::<MappedXWindow>();
        app.add_event::<DWayXWaylandReady>();
        app.add_event::<DWayXWaylandStoped>();
        app.register_relation::<DWayHasXWayland>();
        app.register_relation::<XWindowAttachSurface>();
    }
}
