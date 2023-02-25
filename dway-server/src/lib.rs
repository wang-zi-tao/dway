#![feature(drain_filter)]
pub mod keyboard;
pub mod log;
pub mod math;
pub mod pointer;
pub mod render;
pub mod surface;
pub mod components;
// pub mod wayland;
pub mod wayland_window;
pub mod x11_window;
pub mod placement;
pub mod events;

use bevy_ecs::system::Resource;
use log::logger;
use slog::Logger;
// use wayland::{
//     inputs::{receive_message, receive_messages},
//     render::render_desktop,
// };

// use self::wayland::{CalloopData, DWayState};
use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessage;

pub fn main_loop(receiver: Receiver<WindowMessage>, sender: Sender<WindowMessage>) {
    let log = logger();
    // crate::wayland::backend::udev::run_udev(log,receiver, sender);
}

#[derive(Resource)]
pub struct DWayBackend {
    pub log: Logger,
}

pub struct DWay{

}
