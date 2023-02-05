use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{
    math::{rect_to_rectangle, vec2_to_point},
    wayland::{focus::FocusTarget, surface::with_states_locked},
};

use super::{surface::SurfaceData, DWayState};
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
    prelude::MouseButton,
};
use bevy_math::Vec2;
use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessageKind;
use failure::{format_err, Fallible};
use slog::{debug, error, info, trace, warn};
use smithay::{
    backend::input::ButtonState,
    desktop::space::SpaceElement,
    input::{
        keyboard::FilterResult,
        pointer::{ButtonEvent, MotionEvent},
    },
    output::Scale,
    reexports::{wayland_protocols::xdg::shell::server::xdg_toplevel, wayland_server::Resource},
    utils::{Point, SERIAL_COUNTER},
    xwayland::XwmHandler,
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
                // info!(dway.log, "receiver message: {:?}", &message);
                let uuid = message.uuid;
                trace!(
                    dway.log,
                    "message duration: {:?}",
                    SystemTime::now().duration_since(message.time)
                );
                match message.data {
                    WindowMessageKind::Sync {
                        state,
                        pos,
                        buffer,
                        title,
                    } => todo!(),
                    WindowMessageKind::MouseMove(pos) => {
                        let serial = SERIAL_COUNTER.next_serial();
                        let point = Point::from((pos.x as f64, pos.y as f64));
                        let uuid = &message.uuid;
                        let time =
                            message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
                        if let Some(element) = dway.element_for_uuid(uuid) {
                            dway.seat.get_pointer().unwrap().motion(
                                dway,
                                Some((element.into(), Default::default())),
                                &MotionEvent {
                                    location: point,
                                    serial,
                                    time,
                                },
                            );
                        } else if let Some(surface) = dway.surface_for_uuid(uuid) {
                            if let Some(popup) = dway.popups.find_popup(&surface) {
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
                            }
                        } else {
                            error!(dway.log, "element with {uuid:?} not found, uuid: {uuid:?}");
                            return Ok(());
                        }
                    }
                    WindowMessageKind::MouseButton(MouseButtonInput { button, state }) => {
                        let serial = SERIAL_COUNTER.next_serial();
                        let time =
                            message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
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
                        let time =
                            message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
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
                        key_code,
                        state,
                    }) => {
                        let keyboard = dway.seat.get_keyboard().unwrap();
                        let element = dway.element_for_uuid(&uuid);
                        let serial = SERIAL_COUNTER.next_serial();
                        let time =
                            message.time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u32;
                        if let Some(element) = element {
                            keyboard.set_focus(dway, Some(element.into()), serial);
                        }
                        keyboard.input(
                            dway,
                            // key_code as u32,
                            scan_code,
                            match state {
                                bevy_input::ButtonState::Pressed => {
                                    smithay::backend::input::KeyState::Pressed
                                }
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
                        dway.space
                            .map_element(element.clone(), (pos.x, pos.y), true);
                        let geo = element.geometry().to_f64();
                    }
                    WindowMessageKind::SetRect(geo) => {
                        let geo = rect_to_rectangle(geo).to_i32_round();
                        let Some( element ) = dway.element_for_uuid(&uuid)else{
                            error!(dway.log,"window not found {uuid}");
                            return Ok(());
                        };
                        dway.space.map_element(element.clone(), geo.loc, true);
                        match &element {
                            crate::wayland::shell::WindowElement::Wayland(w) => {
                                let toplevel = w.toplevel();
                                toplevel.with_pending_state(|state| {
                                    state.size = Some(geo.size);
                                });
                                toplevel.send_configure();
                            }
                            crate::wayland::shell::WindowElement::X11(w) => {
                            },
                        };
                    }
                    WindowMessageKind::Normal => {
                        debug!(dway.log, "WindowMessageKind::Normal");
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
                            crate::wayland::shell::WindowElement::X11(_) => {

                            },
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
                            crate::wayland::shell::WindowElement::X11(_) => {

                            },
                        };
                    }
                    WindowMessageKind::Create { pos, size } => todo!(),
                    WindowMessageKind::Destroy => todo!(),
                    WindowMessageKind::Minimized => todo!(),
                    WindowMessageKind::FullScreen => todo!(),
                    _ => {
                        todo!();
                    }
                }
            }
        }
    }
}
