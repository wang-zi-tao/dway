use std::time::Duration;
pub mod keys;

// use dway_client_core::protocol::{WindowMessageReceiver, WindowMessageSender};
// use dway_ui::kayak_ui::{prelude::KayakContextPlugin, widgets::KayakWidgets};

use bevy::{
    app::ScheduleRunnerPlugin,
    audio::AudioPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::CorePipelinePlugin,
    diagnostic::{
        EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    gizmos::GizmoPlugin,
    gltf::GltfPlugin,
    log::{Level, LogPlugin},
    pbr::PbrPlugin,
    prelude::*,
    render::{settings::Backends, RenderPlugin},
    scene::ScenePlugin,
    sprite::SpritePlugin,
    text::TextPlugin,
    ui::UiPlugin,
    winit::WinitPlugin,
};
use bevy_framepace::Limiter;
use dway_client_core::{
    layout::{tile::TileLayoutKind, LayoutRect, LayoutStyle},
    workspace::{Workspace, WorkspaceBundle},
};
use dway_tty::DWayTTYPlugin;
use dway_util::{eventloop::EventLoopPlugin, logger::DWayLogPlugin};
use keys::*;
use tracing_subscriber::{
    fmt::{format::Writer, time::FormatTime},
    EnvFilter,
};

const LOG: &str = "\
bevy_ecs=info,\
bevy_render=debug,\
bevy_ui=trace,\
dway=debug,\
bevy_relationship=debug,\
dway_server=trace,\
dway_server::input=debug,\
dway_server::render=info,\
dway_server::state=info,\
dway_server::wl::buffer=info,\
dway_server::wl::compositor=info,\
dway_server::wl::surface=info,\
dway_server::xdg::popup=debug,\
dway_server::xdg=info,\
nega::front=info,\
naga=warn,\
wgpu=warn,\
";

const THREAD_POOL_CONFIG: TaskPoolThreadAssignmentPolicy = TaskPoolThreadAssignmentPolicy {
    min_threads: 1,
    max_threads: 1,
    percent: 0.25,
};

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::NONE));
    // app.insert_resource(ReportExecutionOrderAmbiguities);
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                wgpu_settings: bevy::render::settings::WgpuSettings {
                    // backends: Some(Backends::GL),
                    ..Default::default()
                },
            })
            .set(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: 1,
                    max_total_threads: 1,
                    io: THREAD_POOL_CONFIG,
                    async_compute: THREAD_POOL_CONFIG,
                    compute: THREAD_POOL_CONFIG,
                },
            })
            .disable::<LogPlugin>()
            .disable::<PbrPlugin>()
            .disable::<GizmoPlugin>()
            .disable::<GltfPlugin>()
            .disable::<ScenePlugin>()
            .disable::<WinitPlugin>()
            .disable::<AudioPlugin>()
            // .disable::<UiPlugin>()
            // .disable::<TextPlugin>()
            .disable::<GilrsPlugin>(),
        // .disable::<RenderPlugin>()
        // .disable::<CorePipelinePlugin>()
        // .disable::<SpritePlugin>()
    );

    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        app.add_plugins((
            DWayLogPlugin {
                level: Level::INFO,
                filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
            },
            DWayTTYPlugin::default(),
            // ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(1.0 / 60.0)),
        ));
    } else {
        // app.add_plugin(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
        // app.insert_resource(dway_winit::WinitSettings {
        //     focused_mode: dway_winit::UpdateMode::ReactiveLowPower {
        //         max_wait: Duration::from_secs_f32(1.0),
        //     },
        //     unfocused_mode: dway_winit::UpdateMode::ReactiveLowPower {
        //         max_wait: Duration::from_secs_f32(1.0),
        //     },
        //     ..Default::default()
        // });
        app.add_plugins((
            LogPlugin {
                level: Level::INFO,
                filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
            },
            EventLoopPlugin::default(),
            WinitPlugin,
            // dway_winit::WinitPlugin,
            bevy_framepace::FramepacePlugin,
        ));
        app.insert_resource(
            bevy_framepace::FramepaceSettings::default()
                .with_limiter(Limiter::from_framerate(60.0)),
        );
    }

    app.add_plugins((
        EntityCountDiagnosticsPlugin,
        FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
        LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(8),
            ..Default::default()
        },
        bevy_inspector_egui::DefaultInspectorConfigPlugin,
    ));

    app.add_plugins((
        dway_server::DWayServerPlugin,
        dway_client_core::DWayClientPlugin,
        dway_ui::DWayUiPlugin,
    ));

    app.add_systems(Startup, setup);
    app.add_systems(Update, (wm_mouse_action, wm_keys));

    app.run();

    // wayland_thread.join().unwrap();
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
        TileLayoutKind::Grid,
        LayoutStyle {
            padding: LayoutRect::new(4),
            ..Default::default()
        },
    ));
}
