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
    },
    gltf::GltfPlugin,
    log::{Level, LogPlugin},
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    scene::ScenePlugin,
    winit::WinitPlugin,
};
use clap::Parser;
use dway_client_core::{
    layout::{LayoutRect, LayoutStyle},
    workspace::{Workspace, WorkspaceBundle, WorkspaceSet},
};
use dway_server::{
    apps::icon::LinuxIconSourcePlugin,
};
use dway_tty::{DWayTTYPlugin, DWayTTYSettings};
use dway_ui_framework::diagnostics::UiDiagnosticsPlugin;
use dway_util::logger::DWayLogPlugin;
use keys::*;
use opttions::DWayOption;
use std::time::Duration;

const LOG_LEVEL: Level = Level::INFO;
const LOG: &str = "\
bevy_ecs=info,\
bevy_render=debug,\
bevy_ui=trace,\
dway=debug,\
polling=info,\
bevy_relationship=debug,\
dway_server=info,\
dway_client_core=info,\
nega::front=info,\
naga=warn,\
wgpu=info,\
wgpu-hal=info,\
dexterous_developer_internal=info,\
bevy_ecss=info,\
dway_tty=info,\
";

#[cfg(not(feature = "hot_reload"))]
pub fn bevy_main() {
    use bevy::prelude::*;

    let mut app = App::new();
    init_app(&mut app, DefaultPlugins.build());
}

#[cfg(feature = "hot_reload")]
use dexterous_developer::{hot_bevy_main, InitialPlugins};

#[cfg(feature = "hot_reload")]
#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    use dexterous_developer::{ReloadMode, ReloadSettings, ReloadableElementPolicy};

    let mut app = App::new();
    app.insert_resource(ReloadSettings {
        display_update_time: true,
        manual_reload: Some(KeyCode::F2),
        toggle_reload_mode: None,
        reload_mode: ReloadMode::Full,
        reloadable_element_policy: ReloadableElementPolicy::All,
        reloadable_element_selection: None,
    });
    init_app(&mut app, initial_plugins.initialize::<DefaultPlugins>());
}

pub fn init_app(app: &mut App, mut default_plugins: PluginGroupBuilder) {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let opts = DWayOption::parse();
    app.insert_resource(opts.clone());
    app.insert_resource(ClearColor(Color::NONE));
    app.insert_resource(Msaa::Sample4);

    #[cfg(feature = "single_thread")]
    {
        default_plugins = default_plugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                max_total_threads: 1,
                ..Default::default()
            },
        });
    }

    #[cfg(all(feature = "dway_log", not(feature = "cpu_profile")))]
    {
        default_plugins = default_plugins.disable::<LogPlugin>().add(DWayLogPlugin {
            level: LOG_LEVEL,
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
        });
    }

    default_plugins = default_plugins
        .set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::VULKAN | Backends::GL),
                priority: bevy::render::settings::WgpuSettingsPriority::Functionality,
                ..Default::default()
            }),
            ..Default::default()
        })
        .add_before::<AssetPlugin, _>(LinuxIconSourcePlugin)
        .disable::<GltfPlugin>()
        .disable::<ScenePlugin>()
        .disable::<WinitPlugin>()
        .disable::<AudioPlugin>()
        .disable::<GilrsPlugin>();

    app.insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs(1)))
        .add_plugins(default_plugins);

    if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        app.insert_resource(DWayTTYSettings {
            frame_duration: Duration::from_secs_f32(1.0 / 144.0),
        });
        app.add_plugins((DWayTTYPlugin::default(),));
    } else {
        app.insert_resource(bevy::winit::WinitSettings::default());
        #[cfg(feature = "eventloop")]
        {
            app.insert_resource(bevy::winit::WinitSettings {
                focused_mode: bevy::winit::UpdateMode::Reactive {
                    wait: Duration::from_secs_f32(1.0),
                },
                unfocused_mode: bevy::winit::UpdateMode::Reactive {
                    wait: Duration::from_secs_f32(1.0),
                },
                return_from_run: true,
            });
        }
        app.add_plugins(WinitPlugin::default());
        #[cfg(feature = "eventloop")]
        {
            app.add_plugins(dway_util::eventloop::EventLoopPlugin::default());
        }
        #[cfg(feature = "debug")]
        {
            app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
        }
    }

    app.add_plugins((
        FrameTimeDiagnosticsPlugin,
        EntityCountDiagnosticsPlugin,
        LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(1),
            ..Default::default()
        },
        UiDiagnosticsPlugin,
    ));

    app.add_plugins((
        dway_server::DWayServerPlugin,
        dway_client_core::DWayClientPlugin,
        dway_ui::DWayUiPlugin,
    ));

    app.add_systems(Startup, setup);
    #[cfg(feature = "debug")]
    {
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
    }
    app.add_systems(Update, (wm_mouse_action, wm_keys, update));
    app.add_systems(Last, last);

    #[cfg(feature = "single_thread")]
    {
        app.edit_schedule(First, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        });
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(Update, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        });
        app.edit_schedule(Last, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        });
    }
    #[cfg(feature = "debug")]
    if opts.debug_schedule {
        debug::print_resources(&mut app.world);
        if let Err(e) = debug::dump_schedules_system_graph(app) {
            error!("failed to dump system graph: {e}");
        }
    }

    app.run();

    info!("exit");
}

pub fn setup(mut commands: Commands) {
    commands
        .spawn((WorkspaceSet, Name::from("WorkspaceSet")))
        .with_children(|c| {
            for i in 0..=9 {
                c.spawn((
                    WorkspaceBundle {
                        workspace: Workspace {
                            name: format!("workspace{i}"),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    LayoutStyle {
                        padding: LayoutRect::new(4),
                        ..Default::default()
                    },
                ));
            }
        });
}

pub fn update(_query: Query<&Window>) {
    // info!("window count: {}",window_query.iter().count());
}

pub fn last(_commands: Commands) {}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
