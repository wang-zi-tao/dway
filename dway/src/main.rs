#![feature(stmt_expr_attributes)]
#[cfg(feature = "debug")]
pub mod debug;
pub mod keys;
pub mod opttions;

use bevy::{
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

fn main() {
    let opts = DWayOption::parse();
    let mut app = App::new();
    app.insert_resource(opts.clone());
    app.insert_resource(ClearColor(Color::NONE));

    let mut default_plugins = DefaultPlugins.build();

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
                .with_limiter(Limiter::from_framerate(1000.0)),
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
            wait_duration: Duration::from_secs(8),
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
    app.add_systems(Update, (wm_mouse_action, wm_keys, update));

    #[cfg(feature = "debug")]
    if opts.debug_schedule {
        debug::print_resources(&mut app.world);
        if let Err(e) = debug::dump_schedules_system_graph(&mut app) {
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

pub fn spawn(
    mut events: EventReader<WaylandDisplayCreated>,
    query: Query<&DWayServer, Added<DWayServer>>,
) {
    for WaylandDisplayCreated(dway_entity, _) in events.iter() {
        if let Ok(compositor) = query.get(*dway_entity) {
            for i in 0..3 {
                let mut command = process::Command::new("gedit");
                command.arg("--new-window");
                compositor.spawn_process(command);
            }

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
                "alacritty",
            ]{
                compositor.spawn_process(process::Command::new(command));
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
            // compositor.spawn_process(command);
        }
    }
}
pub fn spawn_x11(query: Query<&DWayServer>, mut events: EventReader<DWayXWaylandReady>) {
    for DWayXWaylandReady { dway_entity } in events.iter() {
        if let Ok(compositor) = query.get(*dway_entity) {
            // compositor.spawn_process(process::Command::new("glxgears"));
            // compositor.spawn_process_x11(process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/source/gtk+-3.24.37/build/examples/sunny"));
            // compositor.spawn_process_x11(process::Command::new("gnome-system-monitor"));
        }
    }
}

pub fn update(query:Query<&Window>){
    // info!("window count: {}",window_query.iter().count());
}
