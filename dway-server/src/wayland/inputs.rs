use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::math::{ivec2_to_point, rect_to_rectangle};

use super::{surface::DWaySurfaceData, DWayState};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
    prelude::MouseButton,
};

use dway_protocol::window::{WindowMessage, WindowMessageKind};
use failure::Fallible;
use slog::{debug, error, info, trace};
use smithay::{
    backend::input::{ButtonState, InputEvent},
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    reexports::{
        input::DeviceCapability, wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::DisplayHandle,
    },
    utils::{Point, SERIAL_COUNTER},
    wayland::tablet_manager::{TabletDescriptor, TabletSeatTrait},
};

pub fn receive_messages(dway: &mut DWayState, deadline: Instant) -> Fallible<()> {
    loop {
        match dway.receiver.recv_deadline(deadline) {
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => return Ok(()),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                debug!(dway.log, "channel disconnected");
                return Ok(());
            }
            Ok(message) => {
                receive_message(dway, message)?;
            }
        }
    }
}
pub fn receive_message(dway: &mut DWayState, message: WindowMessage) -> Fallible<()> {
    // info!(dway.log, "receiver message: {:?}", &message);
    let uuid = message.uuid;
    trace!(
        dway.log,
        "message duration: {:?}",
        SystemTime::now().duration_since(message.time)
    );
    match message.data {
        WindowMessageKind::Sync {
            state: _,
            pos: _,
            buffer: _,
            title: _,
        } => todo!(),
        WindowMessageKind::MouseMove(pos) => {
            let serial = SERIAL_COUNTER.next_serial();
            let point = Point::from((pos.x as f64, pos.y as f64));
            let uuid = &message.uuid;
            let time = message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
            if let Some(element) = dway.element_for_uuid(uuid) {
                dway.seat.get_pointer().unwrap().motion(
                    dway,
                    Some((element.clone().into(), Default::default())),
                    &MotionEvent {
                        location: point,
                        serial,
                        time,
                    },
                );
            } else if let Some(popup) = dway
                .surface_for_uuid(uuid)
                .and_then(|surface| dway.popups.find_popup(surface))
            {
                popup.geometry();
                dway.seat.get_pointer().unwrap().motion(
                    dway,
                    Some((popup.into(), Default::default())),
                    &MotionEvent {
                        location: point,
                        serial,
                        time,
                    },
                );
            } else {
                error!(dway.log, "element with {uuid:?} not found, uuid: {uuid:?}");
            }
        }
        WindowMessageKind::MouseButton(MouseButtonInput { button, state }) => {
            let serial = SERIAL_COUNTER.next_serial();
            let time = message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
            dway.seat.get_pointer().unwrap().button(
                dway,
                &ButtonEvent {
                    serial,
                    time,
                    button: match button {
                        MouseButton::Left => 0x110,
                        MouseButton::Right => 0x111,
                        MouseButton::Middle => 0x112,
                        MouseButton::Other(o) => o as u32,
                    },
                    state: match state {
                        bevy_input::ButtonState::Pressed => ButtonState::Pressed,
                        bevy_input::ButtonState::Released => ButtonState::Released,
                    },
                },
            );
        }
        WindowMessageKind::MouseWheel(MouseWheel { unit, x, y }) => {
            let time = message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
            dway.seat.get_pointer().unwrap().axis(
                dway,
                smithay::input::pointer::AxisFrame {
                    source: None,
                    time,
                    axis: ((x * 4.0) as f64, (y * 4.0) as f64),
                    discrete: match unit {
                        MouseScrollUnit::Line => None,
                        MouseScrollUnit::Pixel => Some((x as i32, y as i32)),
                    },
                    stop: (false, false),
                },
            );
        }
        WindowMessageKind::KeyboardInput(KeyboardInput {
            scan_code,
            key_code: _,
            state,
        }) => {
            let keyboard = dway.seat.get_keyboard().unwrap();
            let element = dway.element_for_uuid(&uuid);
            let serial = SERIAL_COUNTER.next_serial();
            let time = message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
            if let Some(element) = element {
                keyboard.set_focus(dway, Some(element.clone().into()), serial);
            }
            keyboard.input(
                dway,
                // key_code as u32,
                scan_code,
                match state {
                    bevy_input::ButtonState::Pressed => smithay::backend::input::KeyState::Pressed,
                    bevy_input::ButtonState::Released => {
                        smithay::backend::input::KeyState::Released
                    }
                },
                serial,
                time,
                |_, _, _| FilterResult::<()>::Forward,
            );
        }
        WindowMessageKind::Move(pos) => {
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            let pos = ivec2_to_point(pos);
            // dway.space
            //     .map_element(element.clone(),pos , true);
            // let geo = element.geometry();
            DWaySurfaceData::with_element(element, |s| {
                let delta = s.bbox.loc - s.geo.loc;
                s.geo.loc = pos;
                s.bbox.loc = pos + delta;
            });
            match element {
                crate::wayland::shell::WindowElement::Wayland(_w) => {}
                crate::wayland::shell::WindowElement::X11(w) => {
                    let mut rect = w.geometry();
                    rect.loc = pos;
                    w.configure(Some(rect)).unwrap();
                }
            }
        }
        WindowMessageKind::SetRect(geo) => {
            debug!(dway.log, "WindowMessageKind::SetRect");
            let geo = rect_to_rectangle(geo).to_i32_round();
            // dway.space.map_element(element.clone(), geo.loc, true);
            let Some(surface)=dway.surface_for_uuid(&uuid)else{
                            error!(dway.log,"surface not found {uuid}");
                            return Ok(());
                        };
            DWaySurfaceData::with(surface, |s| {
                let delta = s.bbox.loc - s.geo.loc;
                s.geo = geo;
                s.bbox = geo;
                s.bbox.loc = geo.loc + delta;
            });
            if let Some(element) = dway.element_for_uuid(&uuid) {
                match &element {
                    crate::wayland::shell::WindowElement::Wayland(w) => {
                        let toplevel = w.toplevel();
                        toplevel.with_pending_state(|state| {
                            state.size = Some(geo.size);
                        });
                        toplevel.send_configure();
                    }
                    crate::wayland::shell::WindowElement::X11(_w) => {}
                };
            };
        }
        WindowMessageKind::Normal => {
            if let Some(element) = dway.element_for_uuid(&uuid) {
                debug!(dway.log, "WindowMessageKind::Normal");
                match &element {
                    crate::wayland::shell::WindowElement::Wayland(w) => {
                        let toplevel = w.toplevel();
                        toplevel.with_pending_state(|state| {
                            state.states.unset(xdg_toplevel::State::Maximized);
                            state.states.unset(xdg_toplevel::State::Maximized);
                        });
                        toplevel.send_configure();
                    }
                    crate::wayland::shell::WindowElement::X11(w) => {
                        w.set_maximized(false)?;
                        w.set_minimized(false)?;
                        w.set_fullscreen(false)?;
                    }
                };
            };
        }
        WindowMessageKind::Maximized => {
            debug!(dway.log, "WindowMessageKind::Maximized");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|state| {
                        state.states.set(xdg_toplevel::State::Maximized);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_maximized(true)?;
                }
            };
        }
        WindowMessageKind::Unmaximized => {
            debug!(dway.log, "WindowMessageKind::Unmaximized");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|state| {
                        state.states.unset(xdg_toplevel::State::Maximized);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_maximized(false)?;
                }
            };
        }
        WindowMessageKind::Minimized => {
            debug!(dway.log, "WindowMessageKind::Minimized");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|_state| {
                        // state.states.set(xdg_toplevel::State::Maximized);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_minimized(true)?;
                }
            };
        }
        WindowMessageKind::Unminimized => {
            debug!(dway.log, "WindowMessageKind::Unminimized");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|_state| {
                        // state.states.set(xdg_toplevel::State::Maximized);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_minimized(false)?;
                }
            };
        }
        WindowMessageKind::FullScreen => {
            debug!(dway.log, "WindowMessageKind::Minimized");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|state| {
                        state.states.set(xdg_toplevel::State::Fullscreen);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_fullscreen(true)?;
                }
            };
        }
        WindowMessageKind::UnFullScreen => {
            debug!(dway.log, "WindowMessageKind::UnFullScreen");
            let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
            match &element {
                crate::wayland::shell::WindowElement::Wayland(w) => {
                    let toplevel = w.toplevel();
                    toplevel.with_pending_state(|state| {
                        state.states.unset(xdg_toplevel::State::Fullscreen);
                    });
                    toplevel.send_configure();
                }
                crate::wayland::shell::WindowElement::X11(w) => {
                    w.set_fullscreen(false)?;
                }
            };
        }
        WindowMessageKind::Create { pos: _, size: _ } => todo!(),
        WindowMessageKind::Destroy => todo!(),
        _ => {
            todo!();
        }
    }
    Ok(())
}

pub fn process_input_event(
    dway: &mut DWayState,
    dh: &DisplayHandle,
    event: smithay::backend::input::InputEvent<smithay::backend::libinput::LibinputInputBackend>,
) {
}
