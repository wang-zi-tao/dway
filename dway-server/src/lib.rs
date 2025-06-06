#![feature(option_get_or_insert_default)]
#![feature(async_closure)]
#![feature(ptr_metadata)]
#![feature(trivial_bounds)]
#![feature(iterator_try_collect)]
#![feature(anonymous_pipe)]

use bevy::prelude::*;
// use bevy_tokio_tasks::TokioTasksRuntime;
use dway_util::eventloop::Poller;
use schedule::DWayServerSet;
use state::{create_display, DWayServerConfig, WaylandDisplayCreated};


pub mod apps;
pub mod client;
pub mod dispatch;
pub mod display;
pub mod events;
pub mod geometry;
pub mod input;
pub mod macros;
pub mod prelude;
pub mod render;
pub mod resource;
pub mod schedule;
pub mod state;
pub mod util;
pub mod wl;
pub mod wp;
pub mod x11;
pub mod xdg;
pub mod zwp;
pub mod zxdg;
pub mod zwlr;
pub mod misc;
pub mod clipboard;

#[derive(Default)]
pub struct DWayServerPlugin;
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // bevy_tokio_tasks::TokioTasksPlugin::default(),
            state::DWayStatePlugin,
            client::ClientPlugin,
            geometry::GeometryPlugin,
            schedule::DWayServerSchedulePlugin,
            events::EventPlugin,
            render::DWayServerRenderPlugin,
        ));
        app.add_plugins((
            wl::output::WlOutputPlugin,
            wl::surface::WlSurfacePlugin,
            wl::buffer::WlBufferPlugin,
            wl::region::WlRegionPlugin,
            wl::compositor::WlCompositorPlugin,
            xdg::XdgShellPlugin,
            xdg::toplevel::XdgToplevelPlugin,
            xdg::popup::XdgPopupPlugin,
            zxdg::outputmanager::XdgOutputManagerPlugin,
            zxdg::decoration::DecorationPlugin,
            zwlr::data_control::DataControlPlugin,
            misc::gtk_primary_selection::GtkPrimarySelectionPlugin,
            input::seat::WlSeatPlugin,
        ));
        app.add_plugins((
            wp::PrimarySelectionPlugin,
            x11::DWayXWaylandPlugin,
            zwp::DmaBufferPlugin,
            apps::DesktopEntriesPlugin,
        ));
        app.add_systems(Startup, (init_display, apply_deferred).chain());
    }
}
pub fn init_display(
    mut commands: Commands,
    mut event_sender: EventWriter<WaylandDisplayCreated>,
    config: Res<DWayServerConfig>,
    mut poller: NonSendMut<Poller>,
) {
    create_display(
        &mut commands,
        &config,
        &mut event_sender,
        &mut poller,
    );
}
