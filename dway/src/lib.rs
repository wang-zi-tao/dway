#![feature(stmt_expr_attributes)]
pub mod debug;
pub mod keys;
pub mod opttions;
pub mod spawn_app;

use std::time::Duration;

use bevy::{
    app::PluginGroupBuilder,
    audio::AudioPlugin,
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::schedule::{LogLevel, ScheduleBuildSettings},
    log::{Level, LogPlugin},
    prelude::*,
    render::{
        settings::{Backends, RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    winit::{WakeUp, WinitPlugin},
};
use bevy_framepace::Limiter;
use cfg_if::cfg_if;
use clap::Parser;
use dway_client_core::{
    layout::{
        tile::{TileLayoutKind, TileLayoutSetBuilder},
        LayoutRect, LayoutStyle,
    },
    model::apps::{AppId, AppListModel},
    workspace::{Workspace, WorkspaceBundle, WorkspaceSet},
    DWayClientSetting, OutputType,
};
use dway_server::{apps::{icon::LinuxIconSourcePlugin, launchapp::RunCommandRequest}, xdg::DWayWindow};
use dway_tty::{DWayTTYPlugin, DWayTTYSettings};
use dway_ui_framework::diagnostics::UiDiagnosticsPlugin;
use dway_util::{
    diagnostic::ChangedDiagnosticPlugin,
    logger::{log_layer, DWayLogPlugin},
};
use keys::*;
use opttions::DWayOption;

const LOG_LEVEL: Level = Level::INFO;

const LOG: &str = "\
";

#[cfg(not(feature = "hot_reload"))]
pub fn bevy_main() {
    use bevy::prelude::*;

    let mut app = App::new();
    init_app(&mut app, DefaultPlugins.build());
}

#[cfg(feature = "hot_reload")]
use bevy_dexterous_developer::{self, *};
use spawn_app::spawn_apps_on_launch;

#[cfg(feature = "hot_reload")]
bevy_dexterous_developer::reloadable_main!(bevy_main(initial_plugins) {
    use dexterous_developer::{ReloadMode, ReloadSettings, ReloadableElementPolicy};
    use bevy::prelude::*;

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
});

pub fn init_app(app: &mut App, mut default_plugins: PluginGroupBuilder) {
    #[cfg(feature = "dhat-heap")]
    let _profiler = debug::memory_profiler();

    #[cfg(feature = "pprof")]
    let _pprof_profiler = debug::pprof_profiler();

    #[cfg(feature = "debug_render")]
    let _render_doc_context = {
        debug::start_render_doc()
    };

    app.configure_schedules(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Error,
        hierarchy_detection: LogLevel::Error,
        report_sets: true,
        ..Default::default()
    });

    let opts = DWayOption::parse();
    app.insert_resource(opts.clone());
    app.insert_resource(ClearColor(Color::NONE));
    app.insert_resource(Time::<Fixed>::from_hz(20.0));

    if cfg!(feature = "single_thread") {
        default_plugins = default_plugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                compute: bevy::app::TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: 1,
                    percent: 1.0,
                    on_thread_spawn: None,
                    on_thread_destroy: None,
                },
                ..Default::default()
            },
        });
    }

    let enable_cpu_profile = cfg!(any(feature = "trace_tracy", feature = "trace_chrome"));

    if !enable_cpu_profile {
        default_plugins = default_plugins.add_before::<LogPlugin>(DWayLogPlugin);
    }

    default_plugins = default_plugins
        .set(LogPlugin {
            level: LOG_LEVEL,
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
            custom_layer: if !enable_cpu_profile {
                log_layer
            } else {
                |_| None
            },
        })
        .set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::GL),
                priority: bevy::render::settings::WgpuSettingsPriority::Functionality,
                memory_hints: bevy::render::settings::MemoryHints::MemoryUsage,
                ..Default::default()
            }),
            ..Default::default()
        })
        .set(AssetPlugin {
            file_path: opts.assets.clone(),
            ..Default::default()
        })
        .add_before::<AssetPlugin>(LinuxIconSourcePlugin)
        .disable::<WinitPlugin>()
        .disable::<AudioPlugin>();

    app.insert_resource(Time::<Virtual>::from_max_delta(Duration::from_secs(1)))
        .add_plugins(default_plugins);

    let use_tty = std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err();
    app.insert_resource(DWayClientSetting {
        window_type: if use_tty {
            OutputType::Winit
        } else {
            OutputType::Tty
        },
    });

    if use_tty {
        app.insert_resource(DWayTTYSettings {
            frame_duration: Duration::from_secs_f32(1.0 / opts.frame_rate),
            max_frame_duration: Duration::from_secs(1),
            ..Default::default()
        });
        app.add_plugins((DWayTTYPlugin::default(),));
    } else {
        cfg_if::cfg_if! {
            if #[cfg(any(feature = "cpu_profile", feature="heap_profile"))] {
                app.insert_resource(bevy::winit::WinitSettings::game());
            } else {
                app.insert_resource(bevy::winit::WinitSettings::desktop_app());
                app.insert_resource(bevy_framepace::FramepaceSettings {
                    limiter: Limiter::from_framerate(opts.frame_rate as f64),
                });
            }
        }

        app.add_event::<WakeUp>();
        app.add_plugins((
            WinitPlugin::<WakeUp>::default(),
            dway_util::eventloop::EventLoopPlugin::default(),
            // bevy_framepace::FramepacePlugin,
        ));
    }

    if cfg!(any(feature = "cpu_profile", feature = "heap_profile")) {
        app.add_plugins(LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(8),
            ..Default::default()
        });
    } else {
        app.add_plugins(LogDiagnosticsPlugin {
            wait_duration: Duration::from_secs(1024),
            ..Default::default()
        });
    }

    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin,
        UiDiagnosticsPlugin,
    ));

    app.add_plugins((
        ChangedDiagnosticPlugin::<Transform>::default(),
        ChangedDiagnosticPlugin::<Node>::default(),
    ));

    app.add_plugins((
        dway_server::DWayServerPlugin,
        dway_client_core::DWayClientPlugin,
        dway_ui::DWayUiPlugin,
    ));

    app.add_systems(Startup, setup);
    app.add_systems(Update, (wm_mouse_action, wm_keys, update));
    app.add_systems(Last, last);

    if cfg!(feature = "single_thread") {
        app.edit_schedule(First, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(Update, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        })
        .edit_schedule(Last, |schedule| {
            schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
        });
        if let Some(extract_app) =
            app.get_sub_app_mut(bevy::render::pipelined_rendering::RenderExtractApp)
        {
            extract_app.edit_schedule(ExtractSchedule, |schedule| {
                schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
            });
        };
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.edit_schedule(bevy::render::Render, |schedule| {
                schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
            });
        };
    }

    #[cfg(feature = "dump_system_graph")]
    {
        debug::print_resources(app.world_mut());
        if let Err(e) = debug::dump_schedules_system_graph(app) {
            error!("failed to dump system graph: {e}");
        }
    }

    app.run();

    info!("exit");
}

pub fn setup(
    mut commands: Commands,
    mut app_model: ResMut<AppListModel>,
    opts: Res<DWayOption>,
    mut run_command_request_sender: EventWriter<RunCommandRequest>,
) {
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
                    TileLayoutKind::Float,
                    TileLayoutSetBuilder::default()
                        .layouts(vec![
                            TileLayoutKind::Float,
                            TileLayoutKind::Horizontal,
                            TileLayoutKind::Vertical,
                            TileLayoutKind::Grid,
                            TileLayoutKind::TileLeft { split: 0.6 },
                            TileLayoutKind::Fullscreen,
                        ])
                        .build()
                        .unwrap(),
                ));
            }
        });
    for app in [
        "firefox",
        "code",
        "org.gnome.Nautilus",
        "gnome-system-monitor",
        "org.gnome.Calendar",
        "Alacritty",
        "org.gnome.Console",
    ] {
        app_model.favorite_apps.insert(AppId::from(app.to_string()));
    }

    spawn_apps_on_launch(&opts, &mut run_command_request_sender);
}

pub fn update(_query: Query<&Window>) {
    // info!("window count: {}",window_query.iter().count());
}

pub fn last(_commands: Commands) {
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
