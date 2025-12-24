mod display;
pub mod events;
pub use display::*;
use dway_util::eventloop::Poller;
use systems::update_xwindow_surface;
pub mod screen;
pub mod systems;
pub mod util;
pub mod window;

use crate::{
    client::{Client, ClientEvents},
    prelude::*,
    state::{on_create_display_event, WaylandDisplayCreated},
};

relationship!(XDisplayHasWindow=>XWindowList-<XDisplayRef);

use self::{
    events::dispatch_x11_events,
    systems::{process_window_action_events, x11_window_attach_wl_surface},
    window::{MappedXWindow, XWindow, XWindowAttachSurface},
};

#[derive(Bundle)]
pub struct XWaylandBundle {
    pub display: XWaylandDisplayWrapper,
    pub client: Client,
}

pub fn launch_xwayland(
    mut display_query: Query<&mut DWayServer>,
    mut events: MessageReader<WaylandDisplayCreated>,
    client_events: Res<ClientEvents>,
    poller: NonSendMut<Poller>,
    mut commands: Commands,
) {
    for WaylandDisplayCreated(entity, _) in events.read() {
        if let Ok(mut dway_server) = display_query.get_mut(*entity) {
            if let Err(e) = XWaylandDisplay::spawn(
                &mut dway_server,
                *entity,
                &mut commands,
                &client_events,
                poller.inner().clone(),
            ) {
                error!(error=%e,"failed to launch xwayland");
            };
        }
    }
}

#[derive(Message)]
pub struct DWayXWaylandReady {
    pub dway_entity: Entity,
}

impl DWayXWaylandReady {
    pub fn new(dway_entity: Entity) -> Self {
        Self { dway_entity }
    }
}

#[derive(Message)]
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
            PreUpdate,
            (
                launch_xwayland
                    .run_if(on_event::<WaylandDisplayCreated>)
                    .in_set(DWayServerSet::Create)
                    .after(on_create_display_event),
                dispatch_x11_events
                    .run_if(on_event::<DispatchXWaylandDisplay>)
                    .in_set(DWayServerSet::Dispatch),
                x11_window_attach_wl_surface
                    .run_if(on_event::<XWindowAttachSurfaceRequest>)
                    .in_set(DWayServerSet::UpdateXWayland),
                update_xwindow_surface
                    .run_if(on_event::<XWindowChanged>)
                    .in_set(DWayServerSet::UpdateXWayland)
                    .after(x11_window_attach_wl_surface),
            ),
        );
        app.add_systems(
            Last,
            process_window_action_events
                .run_if(on_event::<WindowAction>)
                .in_set(DWayServerSet::ProcessWindowAction),
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
