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
        DiagnosticsPlugin, FrameTimeDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
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

fn main() {
    // std::env::remove_var("DISPLAY");
    // std::env::set_var("WAYLAND_DISPLAY", "/run/user/1000/wayland-1");
    // let (wayland_sender, client_receiver) = crossbeam_channel::unbounded();
    // let (client_sender, wawyland_receiver) = crossbeam_channel::unbounded();

    // let wayland_thread = thread::Builder::new()
    //     .name("wayland".to_string())
    //     .spawn(move || dway_server::main_loop(wawyland_receiver, wayland_sender))
    //     .unwrap();

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
            .set(LogPlugin {
                level: Level::INFO,
                filter: "dway=trace,dway_server::surface=debug,bevy_ecs=info,wgpu=warn,nega=info,nega::front=info,bevy_render=debug,bevy_ui=trace".to_string(),
            }),
    );
    // app.insert_resource(ClearColor(Color::rgb(0.0, 0.388, 1.0)));
    // app.insert_resource(Time::default());
    // app.insert_resource(WinitSettings::game());
    // app.insert_resource(WinitSettings {
    //     focused_mode: UpdateMode::ReactiveLowPower {
    //         max_wait: Duration::from_secs_f64(1.0 / 70.0),
    //     },
    //     unfocused_mode: UpdateMode::ReactiveLowPower {
    //         max_wait: Duration::from_secs_f64(1.0 / 70.0),
    //     },
    //     return_from_run: true,
    //     // unfocused_mode: UpdateMode::ReactiveLowPower {
    //     //     max_wait: Duration::from_secs(60),
    //     // },
    //     ..Default::default()
    // });
    // app.add_plugins(MinimalPlugins);

    // app
    // //     .add_plugin(CorePlugin {
    // //     task_pool_options: TaskPoolOptions {
    // //         compute: TaskPoolThreadAssignmentPolicy {
    // //             min_threads: 0,
    // //             max_threads: 0,
    // //             percent: 2.0,
    // //         },
    // //         ..CorePlugin::default().task_pool_options
    // //     },
    // // })
    // .add_plugin(LogPlugin {
    //     level: Level::INFO,
    //     filter: "dway=debug,wgpu_core=warn,wgpu_hal=warn".to_string(),
    // })
    //         .add_plugin(TaskPoolPlugin::default())
    //         .add_plugin(TypeRegistrationPlugin::default())
    //         .add_plugin(FrameCountPlugin::default())
    //         .add_plugin(WindowPlugin::default())
    // .add_plugin(TimePlugin::default())
    // .add_plugin(ScheduleRunnerPlugin::default())
    // .add_plugin(TransformPlugin::default())
    // .add_plugin(HierarchyPlugin::default())
    // .add_plugin(DiagnosticsPlugin::default())
    // .add_plugin(InputPlugin::default())
    // // .add_plugin(WindowPlugin {
    // //     window: WindowDescriptor {
    // //         title: "dway".to_string(),
    // //         present_mode: PresentMode::AutoVsync,
    // //         ..default()
    // //     },
    // //     ..default()
    // // })
    // .add_plugin(AssetPlugin::default())
    // .add_plugin(ScenePlugin::default())
    // .add_plugin(WinitPlugin::default())
    // // .insert_resource(WgpuSettings{
    // //         // backends:None,
    // //         ..Default::default()})
    // .add_plugin(RenderPlugin::default())
    // .add_plugin(ImagePlugin::default())
    // .add_plugin(CorePipelinePlugin::default())
    // .add_plugin(SpritePlugin::default())
    // .add_plugin(TextPlugin::default())
    // .add_plugin(UiPlugin::default())
    // .add_plugin(PbrPlugin::default())
    // .add_plugin(GltfPlugin::default())
    // .add_plugin(AudioPlugin::default())
    // .add_plugin(GilrsPlugin::default())
    // .add_plugin(AnimationPlugin::default());

    // app.insert_resource(bevy_framepace::FramepaceSettings {
    //     limiter: bevy_framepace::Limiter::from_framerate(60.0),
    // });
    // app.add_plugin(bevy_framepace::FramepacePlugin);
    //

    // app.add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default());
    app.add_plugin(bevy::diagnostic::EntityCountDiagnosticsPlugin::default());

    // .add_plugin(AssetCountDiagnosticsPlugin::<Image>::default())

    app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    app.add_plugin(SystemInformationDiagnosticsPlugin);

    // app.add_plugin(KayakWidgets);
    // app.add_plugin(KayakContextPlugin);

    app.add_plugin(WorldInspectorPlugin::new());
    // .add_startup_system(hello_world)

    app.add_plugin(dway_server::DWayServerPlugin::default());
    app.add_plugin(dway_client_core::DWayClientPlugin);
    app.add_plugin(dway_ui::DWayUiPlugin);

    // app.insert_resource(WindowMessageReceiver(client_receiver));
    // app.insert_resource(WindowMessageSender(client_sender));

    app.run();

    // wayland_thread.join().unwrap();
}
