use std::{thread, time::Duration};

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_prototype_lyon::{
    prelude::{Fill, GeometryBuilder, ShapeBundle, ShapePlugin},
    render::ShapeMaterial,
    shapes,
};
// use dway_client_core::protocol::{WindowMessageReceiver, WindowMessageSender};
// use dway_ui::kayak_ui::{prelude::KayakContextPlugin, widgets::KayakWidgets};

use bevy::{
    app::ScheduleRunnerPlugin,
    audio::AudioPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::CorePipelinePlugin,
    diagnostic::{
        DiagnosticsPlugin, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        SystemInformationDiagnosticsPlugin,
    },
    gltf::GltfPlugin,
    input::InputPlugin,
    log::{Level, LogPlugin},
    pbr::PbrPlugin,
    prelude::*,
    render::{settings::Backends, RenderPlugin},
    scene::ScenePlugin,
    sprite::{MaterialMesh2dBundle, SpritePlugin},
    text::TextPlugin,
    time::TimePlugin,
    ui::UiPlugin,
    window::PresentMode,
    winit::{UpdateMode, WinitPlugin, WinitSettings},
};

const LOG: &'static str = "\
bevy_ecs=info,\
bevy_render=debug,\
bevy_ui=trace,\
dway=debug,\
dway_server::buffer=info,\
dway_server::input=debug,\
dway_server::render=info,\
dway_server::state=info,\
dway_server::surface=info,\
dway_server::wl::buffer=info,\
dway_server::wl::compositor=debug,\
dway_server::wl::surface=info,\
dway_server::xdg::popup=debug,\
dway_server::xdg=info,\
nega::front=info,\
nega=info,\
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
                    backends: Some(Backends::GL),
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
            .set(WinitPlugin)
            .set(LogPlugin {
                level: Level::INFO,
                filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
            })
            .disable::<PbrPlugin>()
            // .disable::<SpritePlugin>()
            .disable::<GltfPlugin>()
            .disable::<ScenePlugin>()
            .disable::<WinitPlugin>()
            .disable::<GilrsPlugin>(),
    )
    .insert_resource(dway_winit::WinitSettings {
        focused_mode: dway_winit::UpdateMode::ReactiveLowPower {
            max_wait: Duration::from_secs(1),
        },
        unfocused_mode: dway_winit::UpdateMode::ReactiveLowPower {
            max_wait: Duration::from_secs(1),
        },
        ..Default::default()
    })
    .add_plugin(dway_winit::WinitPlugin);
    app.add_plugin(EntityCountDiagnosticsPlugin::default());
    app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    app.add_plugin(SystemInformationDiagnosticsPlugin);

    // app.add_plugin(KayakWidgets);
    // app.add_plugin(KayakContextPlugin);

    app.add_plugin(WorldInspectorPlugin::new());

    app.add_plugin(dway_server::DWayServerPlugin::default());
    app.add_plugin(dway_client_core::DWayClientPlugin);
    app.add_plugin(dway_ui::DWayUiPlugin);

    // app.insert_resource(WindowMessageReceiver(client_receiver));
    // app.insert_resource(WindowMessageSender(client_sender));

    app.run();

    // wayland_thread.join().unwrap();
}
