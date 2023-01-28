pub mod math;
pub mod log;
pub mod wayland;
use std::{
    process::Command,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessage;
use failure::Fallible;
use slog::{error, info, Logger};
use smithay::{
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{calloop::EventLoop, wayland_server::Display},
};
use wayland::{inputs::receive_messages, render::render_desktop};

use self::wayland::{CalloopData, DWayState};

pub fn main_loop(receiver: Receiver<WindowMessage>, sender: Sender<WindowMessage>) {
    let log = log::logger();
    let mut display = Display::new().unwrap();
    let mut event_loop = EventLoop::try_new().unwrap();
    let state = DWayState::init(
        &mut display,
        event_loop.handle(),
        log.clone(),
        receiver,
        sender,
    );
    let mut calloop_data = CalloopData { state, display };

    let mut command = Command::new("alacritty");
    command.args(&["-e", "zsh", "-c", "gnome-system-monitor;zsh"]);
    // command.args(&["-e", "zsh", "-c", "htop;zsh"]);
    // // let command = Command::new("gnome-system-monitor");
    // let command = Command::new("google-ch");
    calloop_data.state.spawn(command);

    while calloop_data.state.running.load(Ordering::SeqCst) {
        let loop_begin = Instant::now();
        let frame_duration = Duration::from_secs_f32(1.0 / 60.0);
        let tick_rate: usize = 8;
        let tick_duration = frame_duration / (tick_rate as u32);
        let result = (|| {
            calloop_data.state.tick();
            loop {
                let now = Instant::now();
                if now < loop_begin + tick_duration / 2 {
                    receive_messages(&mut calloop_data.state, loop_begin + tick_duration / 2)?;
                } else if now < loop_begin + tick_duration {
                    event_loop
                        .dispatch(Some(loop_begin + tick_duration - now), &mut calloop_data)?;
                } else {
                    break;
                }
            }
            // event_loop.dispatch(Some(tick_duration / 2), &mut calloop_data)?;
            if calloop_data.state.tick % tick_rate == 0 {
                calloop_data.display.flush_clients()?;
                calloop_data.state.space.refresh();
                calloop_data.state.popups.cleanup();
                render_desktop(&mut calloop_data.state)?;
                // info!(calloop_data.state.log, "render tick",);
            }
            Fallible::Ok(())
        })();
        if let Err(err) = result {
            error!(calloop_data.state.log, "error in tick: {err}");
        }
        let loop_end = Instant::now();
        // info!(
        //     calloop_data.state.log,
        //     "tick duration: {:?}",
        //     loop_end - loop_begin
        // );
    }
}
