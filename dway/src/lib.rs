#![feature(stmt_expr_attributes)]
#[cfg(feature = "debug")]
pub mod debug;
pub mod keys;
pub mod opttions;
pub mod spawn_app;

use bevy::{
    app::PluginGroupBuilder,
    audio::AudioPlugin,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    gizmos::GizmoPlugin,
    gltf::GltfPlugin,
    log::{Level, LogPlugin},
    pbr::PbrPlugin,
    prelude::*,
    scene::ScenePlugin,
    winit::WinitPlugin,
};
use bevy_framepace::Limiter;
use clap::Parser;
use dway_client_core::{
    layout::{LayoutRect, LayoutStyle},
    workspace::{Workspace, WorkspaceBundle},
};
use dway_server::{
    schedule::DWayServerSet,
    state::{DWayServer, WaylandDisplayCreated},
    x11::DWayXWaylandReady,
};
use dway_tty::DWayTTYPlugin;
use dway_util::{eventloop::EventLoopPlugin, logger::DWayLogPlugin};
use keys::*;
use opttions::DWayOption;
use std::{process, time::Duration};

const LOG_LEVEL: Level = Level::TRACE;
const LOG: &str = "\
bevy_ecs=info,\
bevy_render=debug,\
bevy_ui=trace,\
dway=debug,\
bevy_relationship=debug,\
dway_server=trace,\
dway_server::input=info,\
dway_server::render=info,\
dway_server::state=info,\
dway_server::wl::buffer=info,\
dway_server::wl::compositor=info,\
dway_server::wl::surface=info,\
dway_server::xdg::popup=debug,\
dway_server::xdg=info,\
nega::front=info,\
naga=warn,\
wgpu=trace,\
wgpu-hal=trace
";

#[cfg(not(feature = "dynamic_reload"))]
pub fn bevy_main() {
    use bevy::prelude::*;

    let mut app = App::new();
    init_app(&mut app, DefaultPlugins.build());
}

#[cfg(feature = "dynamic_reload")]
use dexterous_developer::{hot_bevy_main, InitialPlugins};

#[cfg(feature = "dynamic_reload")]
#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    let mut app = App::new();
    init_app(&mut app, initial_plugins.initialize::<DefaultPlugins>());
}

pub fn init_app(app: &mut App, mut default_plugins: PluginGroupBuilder) {
    let opts = DWayOption::parse();
    app.insert_resource(opts.clone());
    app.insert_resource(ClearColor(Color::NONE));

    #[cfg(feature = "single_thread")]
    {
        let thread_pool_config = bevy::core::TaskPoolThreadAssignmentPolicy {
            min_threads: 1,
            max_threads: 1,
            percent: 1.0,
        };
        default_plugins = default_plugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                min_total_threads: 1,
                max_total_threads: 1,
                io: thread_pool_config.clone(),
                async_compute: thread_pool_config.clone(),
                compute: thread_pool_config.clone(),
            },
        });
    }

    #[cfg(feature = "dway_log")]
    {
        default_plugins = default_plugins.disable::<LogPlugin>().add(DWayLogPlugin {
            level: LOG_LEVEL,
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
        });
    }

    #[cfg(feature = "debug")]
    {
        default_plugins =
            default_plugins.add(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }

    default_plugins = default_plugins
        .disable::<PbrPlugin>()
        .disable::<GizmoPlugin>()
        .disable::<GltfPlugin>()
        .disable::<ScenePlugin>()
        .disable::<WinitPlugin>()
        .disable::<AudioPlugin>()
        .disable::<GilrsPlugin>();

    app.add_plugins(default_plugins);

    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        app.add_plugins((DWayTTYPlugin::default(),));
    } else {
        app.insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Reactive {
                wait: Duration::from_secs_f32(1.0),
            },
            unfocused_mode: bevy::winit::UpdateMode::Reactive {
                wait: Duration::from_secs_f32(1.0),
            },
            ..Default::default()
        });
        app.insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
            ..Default::default()
        });
        app.add_plugins((WinitPlugin::default(), bevy_framepace::FramepacePlugin));
        app.insert_resource(
            bevy_framepace::FramepaceSettings::default()
                .with_limiter(Limiter::from_framerate(60.0)),
        );
        #[cfg(feature = "debug")]
        {
            app.add_plugins(EventLoopPlugin::default());
        }
    }

    app.add_plugins((
        EntityCountDiagnosticsPlugin,
        FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
        LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(16),
            ..Default::default()
        },
    ));

    app.add_plugins((
        dway_server::DWayServerPlugin,
        dway_client_core::DWayClientPlugin,
        dway_ui::DWayUiPlugin,
    ));

    app.add_systems(Startup, setup);
    app.add_systems(
        PreUpdate,
        (
            spawn_app::spawn
                .run_if(on_event::<WaylandDisplayCreated>())
                .in_set(DWayServerSet::CreateGlobal),
            spawn_app::spawn_x11
                .run_if(on_event::<DWayXWaylandReady>())
                .in_set(DWayServerSet::UpdateXWayland),
        ),
    );
    app.add_systems(Update, (wm_mouse_action, wm_keys, update));

    #[cfg(feature = "debug")]
    if opts.debug_schedule {
        debug::print_resources(&mut app.world);
        if let Err(e) = debug::dump_schedules_system_graph(app) {
            error!("failed to dump system graph: {e}");
        }
    }

    app.run();
}

pub fn setup(mut commands: Commands) {
    commands.spawn((
        WorkspaceBundle {
            workspace: Workspace {
                name: "workspace0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
        // TileLayoutKind::Grid,
        LayoutStyle {
            padding: LayoutRect::new(4),
            ..Default::default()
        },
    ));
}

pub fn update(query: Query<&Window>) {
    // info!("window count: {}",window_query.iter().count());
}
