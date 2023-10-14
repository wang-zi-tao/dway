#![feature(linked_list_cursors)]

use bevy::prelude::*;
use dway_server::schedule::DWayServerSet;
use log::info;

pub mod components;
pub mod compositor;
pub mod debug;
pub mod desktop;
pub mod input;
pub mod layout;
pub mod navigation;
pub mod prelude;
pub mod screen;
pub mod window;
pub mod workspace;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum DWayClientSystem {
    Init,
    FromServer,
    Create,
    CreateFlush,
    CreateComponent,
    CreateComponentFlush,
    Input,
    UpdateState,
    UpdateFocus,
    UpdateZIndex,
    UpdateLayout,
    UpdateLayoutFlush,
    UpdateWindowGeometry,
    UpdateUI,
    PostUpdate,
    DestroyComponent,
    Destroy,
    DestroyFlush,
    ToServer,
}

#[derive(Hash, Default, Debug, PartialEq, Eq, Clone, States)]
pub enum DWayClientState {
    Init,
    #[default]
    Desktop,
    Locked,
    Overview,
    Fullscreen,
    Moving,
    Resizing,
    Eixt,
}

pub struct DWayClientPlugin;
impl Plugin for DWayClientPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // app.insert_resource(Msaa::Off);
        app.add_state::<DWayClientState>();
        use DWayClientSystem::*;
        app.configure_set(Init);
        app.configure_sets(
            (
                FromServer.after(DWayServerSet::Update),
                Create,
                CreateFlush,
                CreateComponent,
                CreateComponentFlush,
                Input,
                UpdateState,
                UpdateUI,
            )
                .in_base_set(CoreSet::PreUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (UpdateLayout, UpdateLayoutFlush, UpdateWindowGeometry)
                .in_base_set(CoreSet::PreUpdate)
                .chain()
                .after(UpdateState)
                .after(UpdateFocus)
                .before(UpdateUI),
        );
        app.add_system(apply_system_buffers.in_set(UpdateLayoutFlush));
        app.configure_sets(
            (UpdateFocus, UpdateZIndex)
                .in_base_set(CoreSet::PreUpdate)
                .after(UpdateState)
                .before(UpdateUI),
        );
        app.configure_sets(
            (
                PostUpdate,
                DestroyComponent,
                Destroy,
                DestroyFlush,
                ToServer.before(DWayServerSet::PostUpdate),
            )
                .in_base_set(CoreSet::PostUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.add_system(apply_system_buffers.in_set(CreateFlush));
        app.add_system(apply_system_buffers.in_set(CreateComponentFlush));
        app.add_system(apply_system_buffers.in_set(DestroyFlush));

        app.add_plugin(compositor::CompositorPlugin);
        // app.add_plugin(DebugCursorPickingPlugin);
        // app.add_plugin(DebugEventsPickingPlugin);
        // app.add_plugins(DefaultPickingPlugins);
        app.add_startup_system((setup_2d.pipe(apply_system_buffers)).in_set(Init));
        app.add_plugin(input::DWayInputPlugin { debug: false });
        app.add_plugin(desktop::DWayDesktop);
        app.add_plugin(window::DWayWindowPlugin);
        // app.add_plugin(decoration::DWayDecorationPlugin::default());
        app.add_plugin(debug::DebugPlugin::default());
        app.add_plugin(navigation::windowstack::WindowStackPlugin);
        app.add_plugin(layout::LayoutPlugin);
        app.add_plugin(screen::ScreenPlugin);
        app.add_plugin(workspace::WorkspacePlugin);
        // app.add_system(debug_info);
    }
}

pub fn debug_info(cameras: Query<&Camera>, cameras2d: Query<&Camera2d>) {
    info!("cameras : {:?}", cameras.iter().collect::<Vec<_>>());
    info!("cameras2d : {:?}", cameras2d.iter().count());
}
/// set up a simple 2D scene
fn setup_2d() {}
