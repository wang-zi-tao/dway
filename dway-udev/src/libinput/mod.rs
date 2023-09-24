pub mod convert;
pub mod keys;

use anyhow::anyhow;
use std::{
    os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    sync::{Arc, Mutex},
};

use anyhow::Result;
use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::*,
    utils::HashMap,
};
use input::{
    event::{
        keyboard::{KeyboardEventTrait, KeyboardKeyEvent},
        pointer::{Axis, PointerAxisEvent, PointerButtonEvent},
        tablet_pad, EventTrait, KeyboardEvent, PointerEvent, TouchEvent,
    },
    Led, Libinput, LibinputInterface,
};
use libseat::Seat;

use crate::{libinput::convert::convert_keycode, schedule::DWayTTYSet, seat::SeatState};

pub struct SeatLibinputInterface {
    pub(crate) seat: Arc<Mutex<Seat>>,
    pub(crate) devices: HashMap<RawFd, libseat::Device>,
}

impl SeatLibinputInterface {
    pub fn new(seat: Arc<Mutex<Seat>>) -> Self {
        Self {
            seat,
            devices: Default::default(),
        }
    }
}

impl LibinputInterface for SeatLibinputInterface {
    fn open_restricted(
        &mut self,
        path: &std::path::Path,
        flags: i32,
    ) -> std::result::Result<std::os::fd::OwnedFd, i32> {
        let path = path.to_path_buf();
        let device = self.seat.lock().unwrap().open_device(&path)?;
        let fd = device.as_fd().as_raw_fd();
        self.devices.insert(fd, device);
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }

    fn close_restricted(&mut self, fd: std::os::fd::OwnedFd) {
        if let Some(device) = self.devices.remove(&fd.into_raw_fd()) {
            if let Err(e) = self.seat.lock().unwrap().close_device(device) {
                warn!("failed to close device: {e}")
            }
        }
    }
}

pub struct LibinputDevice {
    pub(crate) libinput: Libinput,
}

impl LibinputDevice {
    pub fn new(seat: &mut SeatState) -> Result<Self> {
        let interface = SeatLibinputInterface::new(seat.seat.clone());
        let mut libinput = Libinput::new_with_udev(interface);
        libinput
            .udev_assign_seat(&seat.name)
            .map_err(|e| anyhow!("failed to set seat for libinput"))?;
        info!("libinput connected");
        Ok(Self { libinput })
    }
}

#[derive(Resource, Default)]
pub struct KeyLockState {
    number_lock: bool,
    caps_lock: bool,
    scoll_lock: bool,
}

impl KeyLockState {
    pub(crate) fn led(&self) -> Led {
        let mut led = Led::empty();
        led.set(Led::CAPSLOCK, self.caps_lock);
        led.set(Led::NUMLOCK, self.number_lock);
        led.set(Led::SCROLLLOCK, self.scoll_lock);
        led
    }
}

#[tracing::instrument(skip_all)]
pub fn receive_events(
    mut libinput: NonSendMut<LibinputDevice>,
    mut motion_events: EventWriter<MouseMotion>,
    mut button_events: EventWriter<MouseButtonInput>,
    mut button_state: ResMut<Input<MouseButton>>,
    mut axis_events: EventWriter<MouseWheel>,
    mut keyboard_events: EventWriter<KeyboardInput>,
    mut keycode_state: ResMut<Input<KeyCode>>,
    mut lock_state: ResMut<KeyLockState>,
) {
    button_state.clear();
    keycode_state.clear();
    if let Err(e) = libinput.libinput.dispatch() {
        error!("libinput error: {e}");
    };
    while let Some(event) = libinput.libinput.next() {
        debug!("libinput event: {event:?}");
        match event {
            input::Event::Device(e) => {
                match e {
                    input::event::DeviceEvent::Added(e) => {
                        e.device().led_update(Led::empty());
                        info!("libinput device {e:?} connected");
                    }
                    input::event::DeviceEvent::Removed(e) => {
                        info!("libinput device {e:?} disconnected");
                    }
                    _ => {}
                };
            }
            input::Event::Keyboard(e) => {
                e.device();
                match e {
                    KeyboardEvent::Key(k) => {
                        let key = k.key();
                        let state = k.key_state();
                        keyboard_events.send(KeyboardInput {
                            scan_code: key,
                            key_code: convert_keycode(
                                key,
                                &mut keycode_state,
                                state,
                                &mut lock_state,
                                &mut k.device(),
                            ),
                            state: match state {
                                tablet_pad::KeyState::Pressed => ButtonState::Pressed,
                                tablet_pad::KeyState::Released => ButtonState::Released,
                            },
                        });
                    }
                    _ => {}
                };
            }
            input::Event::Pointer(e) => {
                match e {
                    PointerEvent::Motion(m) => motion_events.send(MouseMotion {
                        delta: DVec2::new(m.dx(), m.dy()).as_vec2(),
                    }),
                    PointerEvent::MotionAbsolute(m) => todo!(),
                    PointerEvent::Button(m) => {
                        let button = match m.button() {
                            0x110 => MouseButton::Left,
                            0x111 => MouseButton::Right,
                            0x112 => MouseButton::Middle,
                            o => {
                                warn!("unknown mouse button: {o}");
                                continue;
                            }
                        };
                        let state = match m.button_state() {
                            tablet_pad::ButtonState::Pressed => ButtonState::Pressed,
                            tablet_pad::ButtonState::Released => ButtonState::Released,
                        };
                        match state {
                            ButtonState::Pressed => button_state.press(button),
                            ButtonState::Released => button_state.release(button),
                        }
                        button_events.send(MouseButtonInput { button, state });
                    }
                    PointerEvent::Axis(m) => axis_events.send(MouseWheel {
                        unit: bevy::input::mouse::MouseScrollUnit::Pixel,
                        x: m.axis_value(Axis::Horizontal) as f32,
                        y: m.axis_value(Axis::Vertical) as f32,
                    }),
                    PointerEvent::ScrollWheel(m) => axis_events.send(MouseWheel {
                        unit: bevy::input::mouse::MouseScrollUnit::Pixel,
                        x: m.scroll_value_v120(Axis::Horizontal) as f32,
                        y: m.scroll_value_v120(Axis::Vertical) as f32,
                    }),
                    PointerEvent::ScrollFinger(m) => {}
                    PointerEvent::ScrollContinuous(m) => {}
                    _ => {}
                };
            }
            input::Event::Touch(e) => {
                match e {
                    TouchEvent::Down(_) => {}
                    TouchEvent::Up(_) => {}
                    TouchEvent::Motion(_) => {}
                    TouchEvent::Cancel(_) => {}
                    TouchEvent::Frame(_) => {}
                    _ => {}
                };
            }
            input::Event::Tablet(e) => {}
            input::Event::TabletPad(e) => {}
            input::Event::Gesture(e) => {}
            input::Event::Switch(e) => {}
            _ => {}
        }
    }
}

pub struct LibInputPlugin;
impl Plugin for LibInputPlugin {
    fn build(&self, app: &mut App) {
        let mut seat = app.world.non_send_resource_mut::<SeatState>();
        let libinput = LibinputDevice::new(&mut seat).unwrap();
        app.insert_non_send_resource(libinput)
            .init_resource::<Input<MouseButton>>()
            .init_resource::<Input<KeyCode>>()
            .init_resource::<KeyLockState>()
            .add_system(receive_events.in_set(DWayTTYSet::LibinputSystem))
            .add_event::<MouseMotion>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseWheel>()
            .add_event::<KeyboardInput>();
    }
}
