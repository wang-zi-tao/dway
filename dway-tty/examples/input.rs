use bevy::{
    app::AppExit,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    log::LogPlugin,
    prelude::*,
};
use dway_tty::{libinput::LibInputPlugin, seat::{SeatPlugin, SeatState}};
use dway_util::eventloop::{EventLoopPlugin, EventLoopPluginMode};
use std::time::Duration;

use tracing::Level;

pub fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.build().add(WindowPlugin::default()))
        .add_plugins((
            EventLoopPlugin {
                mode: EventLoopPluginMode::ManualMode,
            },
            LogPlugin {
                level: Level::DEBUG,
                filter: "".to_string(),
                ..Default::default()
            },
            SeatPlugin,
            LibInputPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, input_event_system);
    app.finish();
    app.cleanup();
    for _i in 0..1024 {
        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0));
    }
}

pub fn setup(mut commands: Commands){
    commands.spawn(Window::default());
}

pub fn input_event_system(
    mut move_events: EventReader<MouseMotion>,
    mut whell_event: EventReader<MouseWheel>,
    mut button_event: EventReader<MouseButtonInput>,
    mut keyboard_event: EventReader<KeyboardInput>,

    mut exit: EventWriter<AppExit>,
) {

    for event in move_events.read() {
        info!("{event:?}");
    }
    for event in whell_event.read() {
        info!("{event:?}");
    }
    for event in button_event.read() {
        info!("{event:?}");
    }
    for event in keyboard_event.read() {
        if event.key_code == KeyCode::Escape {
            exit.send(AppExit::Success);
        }
        info!("{event:?}");
    }
}
