mod display;
pub mod events;
pub use display::*;
pub mod screen;
pub mod util;
pub mod window;
use dway_winit::FrameConditionSchedule;
use crate::{
    client::{Client, ClientEvents},
    prelude::*,
    schedule::DWayServerSet,
    state::{on_create_display_event, DWayDisplay, DWayWrapper, WaylandDisplayCreated},
};

relationship!(XDisplayHasWindow=>XWindowList-<XDisplayRef);

use self::{
    events::{dispatch_x11_events, x11_frame_condition},
    window::{
        process_window_action_events, x11_window_attach_wl_surface, MappedXWindow, XWindow,
        XWindowAttachSurface,
    },
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

#[derive(Event)]
pub struct DWayXWaylandReady {
    pub dway_entity: Entity,
}

impl DWayXWaylandReady {
    pub fn new(dway_entity: Entity) -> Self {
        Self { dway_entity }
    }
}

#[derive(Event)]
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
        if app.get_schedule(FrameConditionSchedule).is_some() {
            app.add_systems(FrameConditionSchedule, x11_frame_condition);
        }
        app.add_systems(
            PreUpdate,
            (
                (
                    launch_xwayland.run_if(on_event::<WaylandDisplayCreated>()),
                    apply_deferred,
                )
                    .chain()
                    .in_set(DWayServerSet::Create)
                    .after(on_create_display_event),
                dispatch_x11_events.in_set(DWayServerSet::Dispatch),
                x11_window_attach_wl_surface.in_set(DWayServerSet::UpdateXWayland),
            ),
        );
        app.add_systems(
            Last,
            process_window_action_events.in_set(DWayServerSet::ProcessWindowAction),
        );
        app.register_type::<XWindow>();
        app.register_type::<MappedXWindow>();
        app.add_event::<DWayXWaylandReady>();
        app.add_event::<DWayXWaylandStoped>();
        app.register_relation::<XDisplayHasWindow>();
        app.register_relation::<DWayHasXWayland>();
        app.register_relation::<XWindowAttachSurface>();
    }
}
