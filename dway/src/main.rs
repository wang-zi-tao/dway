use std::thread;

use dway_client_core::protocol::{WindowMessageReceiver, WindowMessageSender};
use dway_ui::kayak_ui::{prelude::KayakContextPlugin, widgets::KayakWidgets};
use bevy_inspector_egui::WorldInspectorPlugin;

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    log::{Level, LogPlugin},
    prelude::*,
    window::PresentMode,
    winit::WinitSettings, asset::diagnostic::AssetCountDiagnosticsPlugin,
};

fn main() {
    let (wayland_sender, client_receiver) = crossbeam_channel::unbounded();
    let (client_sender, wawyland_receiver) = crossbeam_channel::unbounded();

    let wayland_thread = thread::Builder::new()
        .name("wayland".to_string())
        .spawn(move || dway_server::main_loop(wawyland_receiver, wayland_sender))
        .unwrap();

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.388, 1.0)))
        .insert_resource(Time::default())
        .insert_resource(WinitSettings::game())
        // .insert_resource(WinitSettings {
        //     focused_mode: bevy::winit::UpdateMode::ReactiveLowPower {
        //         max_wait: Duration::from_secs_f64(1.0 / 60.0),
        //     },
        //     unfocused_mode: bevy::winit::UpdateMode::ReactiveLowPower {
        //         max_wait: Duration::from_secs_f64(1.0 / 60.0),
        //     },
        //     ..default()
        // })
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: Level::INFO,
                    filter: "info,dway=debug,wgpu_core=warn".to_string(),
                })
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "dway".to_string(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    },
                    ..default()
                }),
        )
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(EntityCountDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(AssetCountDiagnosticsPlugin::<Image>::default())
        .add_plugin(KayakWidgets)
        .add_plugin(KayakContextPlugin)
        // .add_plugin(WorldInspectorPlugin::new())
        // .add_startup_system(hello_world)
        .add_plugin(dway_client_core::WaylandPlugin)
        .add_plugin(dway_ui::DWayUiPlugin)
        .insert_resource(WindowMessageReceiver(client_receiver))
        .insert_resource(WindowMessageSender(client_sender))
        .run();

    // wayland_thread.join().unwrap();
}
