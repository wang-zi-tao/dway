mod display;
pub mod events;
use std::sync::Arc;

pub use display::*;
pub mod screen;
pub mod util;
pub mod window;
use dway_winit::FrameConditionSchedule;

use crate::{
    client::{self, Client, ClientData},
    prelude::*,
    schedule::DWayServerSet,
    state::{on_create_display_event, DWayDisplay, DWayWrapper, DisplayCreated},
};

relationship!(XDisplayHasWindow=>XWindowList-<XDisplayRef);

use self::{
    events::{dispatch_x11_events, x11_frame_condition},
    window::{x11_window_attach_wl_surface, XWindow, MappedXWindow},
};

#[derive(Bundle)]
pub struct XWaylandBundle {
    pub display: XWaylandDisplayWrapper,
    pub client: Client,
}

pub fn launch_xwayland(
    display_query: Query<(&DWayWrapper, &DWayDisplay)>,
    mut events: EventReader<DisplayCreated>,
    mut commands: Commands,
) {
    for DisplayCreated(entity, _) in events.iter() {
        if let Ok((dway_wrapper, display_wrapper)) = display_query.get(*entity) {
            let mut dway = dway_wrapper.0.lock().unwrap();
            let display = display_wrapper.0.lock().unwrap();
            match XWaylandDisplay::new(&mut dway) {
                Ok((xwayland, wayland_client_stream)) => {
                    let mut entity_mut = commands.spawn((
                        Name::new(format!("xwayland:{}", xwayland.display_number)),
                        XWaylandDisplayWrapper::new(xwayland),
                    ));
                    let client = match display.handle().insert_client(
                        wayland_client_stream,
                        Arc::new(ClientData::new(entity_mut.id())),
                    ) {
                        Ok(o) => o,
                        Err(e) => {
                            error!(error=%e);
                            continue;
                        }
                    };
                    entity_mut.insert(client::Client::new(client));
                    entity_mut.set_parent(*entity);
                }
                Err(e) => {
                    error!(error=%e,"failed to launch xwayland");
                }
            };
        }
    }
}

pub struct DWayXWaylandPlugin;
impl Plugin for DWayXWaylandPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                launch_xwayland.run_if(on_event::<DisplayCreated>()),
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
        app.register_type::<XWindow>();
        app.register_type::<MappedXWindow>();
    }
}
