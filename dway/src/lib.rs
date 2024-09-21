#![feature(stmt_expr_attributes)]
pub mod debug;
pub mod keys;
pub mod opttions;
pub mod spawn_app;

use std::{
    path::{absolute, PathBuf},
    time::Duration,
};

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
    window::RequestRedraw,
    winit::{UpdateMode, WakeUp, WinitPlugin, WinitSettings},
};
use bevy_framepace::Limiter;
use cfg_if::cfg_if;
use clap::Parser;
use dway_client_core::{
    layout::{
        tile::{TileLayoutKind, TileLayoutSet, TileLayoutSetBuilder},
        LayoutRect, LayoutStyle,
    },
    model::apps::{AppId, AppListModel},
    workspace::{Workspace, WorkspaceBundle, WorkspaceSet},
    DWayClientSetting, OutputType,
};
use dway_server::apps::icon::LinuxIconSourcePlugin;
use dway_tty::{DWayTTYPlugin, DWayTTYSettings};
use dway_ui_framework::diagnostics::UiDiagnosticsPlugin;
use dway_util::logger::{log_layer, DWayLogPlugin};
use keys::*;
use opttions::DWayOption;

cfg_if! {if #[cfg(feature="debug")]{
    const LOG_LEVEL: Level = Level::DEBUG;
}else{
    const LOG_LEVEL: Level = Level::INFO;
}}

const LOG: &str = "\
bevy_ecs=info,\
bevy_render=debug,\
bevy_ui=trace,\
bevy_time=info,\
dway=debug,\
polling=info,\
bevy_relationship=info,\
dway_server=debug,\
dway_server::render::importnode=debug,\
dway_server::zxdg::decoration=debug,\
dway_client_core=info,\
dway_util::eventloop=info,\
dway_tty=info,\
nega::front=info,\
naga=warn,\
wgpu=info,\
wgpu-hal=info,\
dexterous_developer_internal=debug,\
bevy_ecss=info,\
";

#[cfg(not(feature = "hot_reload"))]
pub fn bevy_main() {
    use bevy::prelude::*;

    let mut app = App::new();
    init_app(&mut app, DefaultPlugins.build());
}

#[cfg(feature = "hot_reload")]
use bevy_dexterous_developer::{self, *};

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

    app.configure_schedules(ScheduleBuildSettings {
        ambiguity_detection: LogLevel::Warn,
        hierarchy_detection: LogLevel::Warn,
        report_sets: true,
        ..Default::default()
    });

    let opts = DWayOption::parse();
    app.insert_resource(opts.clone());
    app.insert_resource(ClearColor(Color::NONE));
    app.insert_resource(Msaa::Sample4);

    if cfg!(feature = "single_thread") {
        default_plugins = default_plugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions {
                compute: bevy::core::TaskPoolThreadAssignmentPolicy {
                    min_threads: 1,
                    max_threads: 1,
                    percent: 1.0,
                },
                ..Default::default()
            },
        });
    }

    default_plugins = default_plugins
        .add_before::<LogPlugin, _>(DWayLogPlugin)
        .set(LogPlugin {
            level: LOG_LEVEL,
            filter: std::env::var("RUST_LOG").unwrap_or_else(|_| LOG.to_string()),
            custom_layer: log_layer,
        })
        .set(RenderPlugin {
            render_creation: RenderCreation::Automatic(WgpuSettings {
                backends: Some(Backends::GL),
                priority: bevy::render::settings::WgpuSettingsPriority::Functionality,
                ..Default::default()
            }),
            ..Default::default()
        })
        .set(AssetPlugin {
            file_path: opts.assets.clone(),
            ..Default::default()
        })
        .add_before::<AssetPlugin, _>(LinuxIconSourcePlugin)
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

        app.add_plugins((
            WinitPlugin::<WakeUp>::default(),
            dway_util::eventloop::EventLoopPlugin::default(),
            // bevy_framepace::FramepacePlugin,
        ));
        //#[cfg(feature = "inspector")]
        //{
        //    app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
        //    app.add_plugins(bevy_inspector_egui::quick::FilterQueryInspectorPlugin::<
        //        With<dway_ui::widgets::window::WindowUI>,
        //    >::default()); //TODO
        //}
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
        FrameTimeDiagnosticsPlugin,
        EntityCountDiagnosticsPlugin,
        UiDiagnosticsPlugin,
    ));

    app.add_plugins((
        dway_server::DWayServerPlugin,
        dway_client_core::DWayClientPlugin,
        dway_ui::DWayUiPlugin,
    ));

    app.add_systems(Startup, setup);
    if cfg!(feature = "debug") {
        app.observe(spawn_app::spawn);
        app.observe(spawn_app::spawn_x11);
    }
    app.add_systems(Update, (wm_mouse_action, wm_keys, update));
    app.add_systems(Last, last);

    if cfg!(feature = "single_thread") {
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
    app.edit_schedule(First, |schedule| {
        schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
    });
    app.edit_schedule(Last, |schedule| {
        schedule.set_executor_kind(bevy::ecs::schedule::ExecutorKind::SingleThreaded);
    });

    #[cfg(feature = "dump_system_graph")]
    if opts.debug_schedule {
        debug::print_resources(app.world_mut());
        if let Err(e) = debug::dump_schedules_system_graph(app) {
            error!("failed to dump system graph: {e}");
        }
    }
    #[cfg(feature = "debug")]
    {
        app.add_systems(
            PreUpdate,
            debug::print_debug_info
                .after(bevy::ui::UiSystem::Focus)
                .before(dway_client_core::input::on_input_event),
        );
    }

    app.run();

    info!("exit");
}

pub fn setup(mut commands: Commands, mut app_model: ResMut<AppListModel>) {
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
}

pub fn update(_query: Query<&Window>) {
    // info!("window count: {}",window_query.iter().count());
}

pub fn last(_commands: Commands) {
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;
