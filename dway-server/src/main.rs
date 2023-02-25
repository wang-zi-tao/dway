pub mod log;
pub mod math;
pub mod wayland;
use dway_server::main_loop;
use log::logger;
use wayland::{
    inputs::{receive_message, receive_messages},
    render::render_desktop,
};

use crossbeam_channel::{Receiver, Sender};
use self::wayland::{CalloopData, DWayState};
use dway_protocol::window::WindowMessage;


fn main(){

    let (wayland_sender, client_receiver) = crossbeam_channel::unbounded();
    let (client_sender, wawyland_receiver) = crossbeam_channel::unbounded();
    main_loop(wawyland_receiver,wayland_sender);
}
