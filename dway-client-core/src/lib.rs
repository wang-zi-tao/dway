#![feature(linked_list_cursors)]

use bevy::{prelude::*, time::TimeSystem};
use bevy_relationship::{relationship, AppExt};
use dway_server::schedule::DWayServerSet;
use dway_util::tokio::TokioPlugin;
use log::info;
use smart_default::SmartDefault;

pub mod components;
pub mod compositor;
pub mod config;
pub mod controller;
pub mod desktop;
pub mod input;
pub mod layout;
pub mod model;
pub mod navigation;
pub mod prelude;
pub mod screen;
pub mod window;
pub mod workspace;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum DWayClientSystem {
    Init,
    CreateScreen,
    InsertWindowComponent,
    Input,
    UpdateSystemInfo,
    UpdateState,
    UpdateFocus,
    UpdateWindowStack,
    UpdateZIndex,
    UpdateLayout,
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
        if !app.is_plugin_added::<TokioPlugin>() {
            app.add_plugins(TokioPlugin::default());
        }
        app.configure_sets(FixedFirst, UpdateSystemInfo.after(TimeSystem));
        app.configure_sets(
            PreUpdate,
            (
                InsertWindowComponent,
                CreateScreen,
                UpdateWindowStack,
                UpdateFocus,
                UpdateZIndex
                    .after(InsertWindowComponent)
                    .after(UpdateFocus)
                    .after(UpdateWindowStack),
                UpdateScreen.after(InsertWindowComponent),
                UpdateWindow.after(UpdateScreen),
                UpdateWorkspace.after(UpdateScreen),
                UpdateLayout
                    .after(UpdateWorkspace)
                    .after(UpdateScreen)
                    .after(UpdateWindowGeometry),
                UpdateWindowGeometry.before(UpdateLayout),
                UpdateState,
            )
                .ambiguous_with_all()
                .before(UpdateUI)
                .after(DWayServerSet::UpdateGeometry),
        );
        app.configure_sets(
            PostUpdate,
            (
                Input,
                DestroyComponent,
                Destroy,
                DestroyFlush,
                ToServer.before(DWayServerSet::EndPreUpdate),
            )
                .chain()
                .before(DWayServerSet::StartPostUpdate)
                .ambiguous_with_all(),
        );

        app.add_systems(PostUpdate, apply_deferred.in_set(DestroyFlush));
        app.add_plugins((
            model::DWayClientModelPlugin,
            controller::ControllerPlugin::default(),
            compositor::CompositorPlugin,
            input::DWayInputPlugin { debug: false },
            desktop::DWayDesktop,
            window::DWayWindowPlugin,
            navigation::windowstack::WindowStackPlugin,
            layout::LayoutPlugin,
            screen::ScreenPlugin,
            workspace::WorkspacePlugin,
        ));

        app.register_relation::<UiAttachData>();
    }
}

relationship!(UiAttachData=>DataRef>-<UiList);

pub fn debug_info(cameras: Query<&Camera>, cameras2d: Query<&Camera2d>) {
    info!("cameras : {:?}", cameras.iter().collect::<Vec<_>>());
    info!("cameras2d : {:?}", cameras2d.iter().count());
}
