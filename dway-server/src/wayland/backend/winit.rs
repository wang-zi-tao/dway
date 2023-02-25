use std::{
    process::Command,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use crate::{
    log,
    wayland::{inputs::receive_message, render::render_desktop, CalloopData, DWayState},
};
use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessage;
use failure::Fallible;
use slog::{debug, error, log};
use smithay::{
    backend::{
        renderer::{damage::DamageTrackedRenderer, gles2::Gles2Renderer},
        winit::WinitGraphicsBackend,
    },
    reexports::{calloop::EventLoop, wayland_server::Display},
    wayland::dmabuf::{DmabufGlobal, DmabufState},
};

#[derive(Debug)]
pub struct WinitBackend {
    pub backend: WinitGraphicsBackend<Gles2Renderer>,
    pub damage_tracked_renderer: DamageTrackedRenderer,
    pub dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub full_redraw: u8,
}

pub fn main_loop(receiver: Receiver<WindowMessage>, sender: Sender<WindowMessage>) {
    let log = log::logger();
    let mut display = Display::new().unwrap();
    let mut event_loop = EventLoop::try_new().unwrap();
    let state = DWayState::init(
        &mut display,
        event_loop.handle(),
        crate::wayland::backend::Backend::Headless,
        log.clone(),
        receiver,
        sender,
    );
    let mut calloop_data = CalloopData { state, display };

    let mut command = Command::new("alacritty");
    // command.args(&["-e", "zsh", "-c", "gnome-calculator;zsh"]);
    // command.args(["-e", "zsh", "-c", "sleep 1;DISPLAY=:2 glxgears;zsh"]);
    // command.args(&["-e", "zsh", "-c", "gnome-system-monitor;zsh"]);
    // // let command = Command::new("gnome-system-monitor");
    // let command = Command::new("google-ch");
    calloop_data.state.spawn(command);

    while calloop_data.state.running.load(Ordering::SeqCst) {
        let loop_begin = Instant::now();
        let frame_rate = 60;
        let frame_duration = Duration::from_secs_f32(1.0 / frame_rate as f32);
        let tick_rate: usize = 2;
        let tick_duration = frame_duration / (tick_rate as u32);
        let result = (|| {
            calloop_data.state.tick();
            loop {
                let now = Instant::now();
                if now < loop_begin + tick_duration / 2 {
                    let dway: &mut DWayState = &mut calloop_data.state;
                    let deadline = loop_begin + tick_duration / 2;
                    loop {
                        match dway.receiver.recv_deadline(deadline) {
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => break,
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                                debug!(dway.log, "channel disconnected");
                                calloop_data.state.running.store(false, Ordering::SeqCst);
                                break;
                            }
                            Ok(message) => {
                                receive_message(dway, message)?;
                            }
                        }
                    }
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
                // calloop_data.state.space.refresh();
                calloop_data.state.popups.cleanup();
                render_desktop(&mut calloop_data.state)?;
                // info!(calloop_data.state.log, "render tick",);
            }
            if calloop_data.state.tick % (tick_rate * frame_rate) == 0 {
                calloop_data.state.debug();
            }
            Fallible::Ok(())
        })();
        if let Err(err) = result {
            error!(calloop_data.state.log, "error in tick: {err}");
        }

        let _loop_end = Instant::now();
        // info!(
        //     calloop_data.state.log,
        //     "tick duration: {:?}",
        //     loop_end - loop_begin
        // );
    }
}
