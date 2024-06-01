#![feature(linked_list_cursors)]

use bevy::prelude::*;
use bevy_relationship::relationship;
use dway_server::schedule::DWayServerSet;
use dway_util::tokio::TokioPlugin;
use log::info;
use smart_default::SmartDefault;

pub mod components;
pub mod compositor;
pub mod config;
pub mod controller;
pub mod model;
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
    UpdateScreen,
    UpdateWorkspace,
    UpdateWindow,
    UpdateUI,
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

structstruck::strike! {
    #[derive(Resource, Clone, Reflect, SmartDefault)]
    pub struct DWayClientSetting {
        pub window_type: #[derive(Clone, SmartDefault, Reflect)]
        pub enum OutputType {
            Winit,
            #[default]
            Tty,
        }
    }
}

/// dway client plugin
///
///
pub struct DWayClientPlugin;
impl Plugin for DWayClientPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_state::<DWayClientState>();
        app.init_resource::<DWayClientSetting>();
        use DWayClientSystem::*;
        app.configure_sets(Startup, Init);
        if !app.is_plugin_added::<TokioPlugin>(){
            app.add_plugins(TokioPlugin::default());
        }
        app.configure_sets(
            PreUpdate,
            (
                FromServer,
                Create,
                CreateFlush,
                CreateComponent,
                CreateComponentFlush,
                Input,
                UpdateState,
                UpdateUI,
            )
                .chain()
                .after(DWayServerSet::EndPreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (UpdateLayout, UpdateLayoutFlush, UpdateWindowGeometry)
                .chain()
                .after(DWayServerSet::EndPreUpdate)
                .after(UpdateState)
                .after(UpdateFocus)
                .before(UpdateUI),
        );
        app.configure_sets(
            PreUpdate,
            (UpdateScreen, UpdateWorkspace, UpdateWindow)
                .after(DWayServerSet::EndPreUpdate)
                .after(UpdateState)
                .before(UpdateUI),
        );
        app.configure_sets(
            PreUpdate,
            (UpdateFocus, UpdateZIndex)
                .after(DWayServerSet::EndPreUpdate)
                .after(UpdateState)
                .before(UpdateUI),
        );
        app.configure_sets(
            PostUpdate,
            (
                DestroyComponent,
                Destroy,
                DestroyFlush,
                ToServer.before(DWayServerSet::EndPreUpdate),
            )
                .chain()
                .before(DWayServerSet::StartPostUpdate)
                .ambiguous_with_all(),
        );

        app.add_systems(
            PreUpdate,
            (
                apply_deferred.in_set(UpdateLayoutFlush),
                apply_deferred.in_set(CreateFlush),
                apply_deferred.in_set(CreateComponentFlush),
            ),
        );

        app.add_systems(PreUpdate, (setup_2d, apply_deferred).chain().in_set(Init));
        app.add_systems(PostUpdate, apply_deferred.in_set(DestroyFlush));
        app.add_plugins((
            model::DWayClientModelPlugin,
            controller::ControllerPlugin::default(),
            compositor::CompositorPlugin,
            input::DWayInputPlugin { debug: false },
            desktop::DWayDesktop,
            window::DWayWindowPlugin,
            debug::DebugPlugin::default(),
            navigation::windowstack::WindowStackPlugin,
            layout::LayoutPlugin,
            screen::ScreenPlugin,
            workspace::WorkspacePlugin,
        ));
    }
}

relationship!(UiAttachData=>DataRef>-<UiList);

pub fn debug_info(cameras: Query<&Camera>, cameras2d: Query<&Camera2d>) {
    info!("cameras : {:?}", cameras.iter().collect::<Vec<_>>());
    info!("cameras2d : {:?}", cameras2d.iter().count());
}
fn setup_2d() {}
