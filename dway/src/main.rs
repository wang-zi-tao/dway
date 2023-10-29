use std::{process, time::Duration};
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
    winit::{UpdateMode, WinitPlugin, WinitSettings},
};
use bevy_framepace::Limiter;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use dway_client_core::{
    layout::{tile::TileLayoutKind, LayoutRect, LayoutStyle},
    workspace::{Workspace, WorkspaceBundle},
};
use dway_server::{
    schedule::DWayServerSet,
    state::{DWayServer, TokioTasksRuntime, WaylandDisplayCreated},
    x11::DWayXWaylandReady,
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
    app.add_plugins((
        DWayLogPlugin {
            level: Level::INFO,
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
        },
        DefaultPlugins
            .build()
            // .set(TaskPoolPlugin {
            //     task_pool_options: TaskPoolOptions {
            //         min_total_threads: 1,
            //         max_total_threads: 1,
            //         io: THREAD_POOL_CONFIG,
            //         async_compute: THREAD_POOL_CONFIG,
            //         compute: THREAD_POOL_CONFIG,
            //     },
            // })
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
    ));

    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        app.add_plugins((DWayTTYPlugin::default(),));
    } else {
        // app.insert_resource(WinitSettings::desktop_app());
        // app.insert_resource(WinitSettings {
        //     focused_mode: UpdateMode::Reactive {
        //         max_wait: Duration::from_secs_f32(1.0),
        //     },
        //     unfocused_mode: UpdateMode::Reactive {
        //         max_wait: Duration::from_secs_f32(1.0),
        //     },
        //     ..Default::default()
        // });
        app.add_plugins((
            EventLoopPlugin::default(),
            WinitPlugin,
            bevy_framepace::FramepacePlugin,
            WorldInspectorPlugin::new(),
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
            wait_duration: Duration::from_secs(32),
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
            spawn
                .run_if(on_event::<WaylandDisplayCreated>())
                .in_set(DWayServerSet::CreateGlobal),
            spawn_x11
                .run_if(on_event::<DWayXWaylandReady>())
                .in_set(DWayServerSet::UpdateXWayland),
        ),
    );
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

pub fn spawn(
    mut events: EventReader<WaylandDisplayCreated>,
    query: Query<&DWayServer, Added<DWayServer>>,
    tokio: Res<TokioTasksRuntime>,
) {
    for WaylandDisplayCreated(dway_entity, _) in events.iter() {
        if let Ok(compositor) = query.get(*dway_entity) {
            // for i in 0..8 {
            //     let mut command = process::Command::new("gedit");
            //     command.arg("--new-window");
            //     compositor.spawn_process(command, &tokio);
            // }

            for command in [
                "gnome-system-monitor",
                "gedit",
                "gnome-calculator",
                "gnome-clocks",
                "gnome-disks",
                "gnome-logs",
                "gnome-music",
                "gnome-maps",
                "gnome-photos",
                "gnome-text-editor",
                "gnome-tweaks",
                "gnome-weather",
                "/home/wangzi/.build/5e0dff7f0473a25a4eb0bbaeeda9b3fa091ba89-wgpu/debug/examples/cube",
            ]{
                compositor.spawn_process(process::Command::new(command), &tokio);
            }

            // let mut command = process::Command::new("alacritty");
            // command.args(["-e", "htop"]);
            // command.current_dir("/home/wangzi/workspace/waylandcompositor/conrod/");
            // let mut command = process::Command::new("/nix/store/gfn9ya0rwaffhfkpbbc3pynk247xap1h-qt5ct-1.5/bin/qt5ct");
            // let mut command = process::Command::new("/home/wangzi/.build/0bd4966a8a745859d01236fd5f997041598cc31-bevy/debug/examples/animated_transform");
            // let mut command = process::Command::new( "/home/wangzi/workspace/waylandcompositor/winit_demo/target/debug/winit_demo",);
            // let mut command = process::Command::new("/home/wangzi/workspace/waylandcompositor/wayland-rs/wayland-client/../target/debug/examples/simple_window");
            // let mut command = process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/GTK-Demo-Examples/guidemo/00_hello_world_classic/hello_world_classic");
            // let mut command =
            //     process::Command::new("/home/wangzi/Code/winit/target/debug/examples/window_debug");
            // compositor.spawn_process(command, &tokio);
        }
    }
}
pub fn spawn_x11(
    query: Query<&DWayServer>,
    tokio: Res<TokioTasksRuntime>,
    mut events: EventReader<DWayXWaylandReady>,
) {
    for DWayXWaylandReady { dway_entity } in events.iter() {
        if let Ok(compositor) = query.get(*dway_entity) {
            // compositor.spawn_process(process::Command::new("glxgears"), &tokio);
            // compositor.spawn_process_x11(process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/source/gtk+-3.24.37/build/examples/sunny"), &tokio);
            // compositor.spawn_process_x11(process::Command::new("gnome-system-monitor"), &tokio);
        }
    }
}
