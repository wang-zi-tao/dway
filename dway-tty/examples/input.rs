use std::time::Duration;
use bevy::{
    core::TaskPoolThreadAssignmentPolicy,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    log::LogPlugin,
    prelude::*,
};
use dway_tty::{
    libinput::LibInputPlugin, seat::SeatPlugin,
};

use tracing::Level;

pub fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin {
            level: Level::INFO,
            filter: "info".to_string(),
        })
        .add_plugin(SeatPlugin)
        .add_plugin(LibInputPlugin)
        .add_system(input_event_system);
    app.finish();
    app.cleanup();
    for i in 0..256 {
        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0));
    }
}

pub fn input_event_system(
    mut move_events: EventReader<MouseMotion>,
    mut whell_event: EventReader<MouseWheel>,
    mut button_event: EventReader<MouseButtonInput>,
    mut keyboard_event: EventReader<KeyboardInput>,
) {
    for event in move_events.iter() {
        dbg!(event);
    }
    for event in whell_event.iter() {
        dbg!(event);
    }
    for event in button_event.iter() {
        dbg!(event);
    }
    for event in keyboard_event.iter() {
        dbg!(event);
    }
}
