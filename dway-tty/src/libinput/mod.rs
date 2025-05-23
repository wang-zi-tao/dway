pub mod convert;

use anyhow::anyhow;
use dway_util::eventloop::{Poller, PollerGuard};
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
        keyboard::KeyboardEventTrait, pointer::Axis, tablet_pad, EventTrait, KeyboardEvent,
        PointerEvent, TouchEvent,
    },
    Led, Libinput, LibinputInterface,
};
use libseat::Seat;

use crate::{
    libinput::convert::convert_keycode, schedule::DWayTTYSet,
    seat::SeatState, window::relative_to_window,
};

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
        _flags: i32,
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

#[derive(Deref)]
pub struct LibinputDevice {
    #[deref]
    pub(crate) libinput: PollerGuard<Libinput>,
    pub mouse_speed: f32,
    pub mouse_wheel_speed: f32,
}

impl AsRawFd for LibinputDevice {
    fn as_raw_fd(&self) -> RawFd {
        self.libinput.as_raw_fd()
    }
}

impl LibinputDevice {
    pub fn new(seat: &mut SeatState, poller: &mut Poller) -> Result<Self> {
        let interface = SeatLibinputInterface::new(seat.seat.clone());
        let mut libinput = Libinput::new_with_udev(interface);
        libinput
            .udev_assign_seat(&seat.name)
            .map_err(|e| anyhow!("failed to set seat for libinput: {e:?}"))?;
        info!("libinput connected");
        Ok(Self {
            libinput: poller.add(libinput),
            mouse_speed: 1.0,
            mouse_wheel_speed: 1.0,
        })
    }
}

#[derive(Resource, Default, Reflect)]
pub struct KeyLockState {
    number_lock: bool,
    caps_lock: bool,
    scoll_lock: bool,
}

#[derive(Resource, Default, Reflect)]
pub struct PointerState {
    pub position: Vec2,
    pub window: Option<Entity>,
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
    mut windows: Query<(Entity, &mut Window)>,
    mut libinput: NonSendMut<LibinputDevice>,
    mut motion_events: EventWriter<MouseMotion>,
    mut move_events: EventWriter<CursorMoved>,
    mut button_events: EventWriter<MouseButtonInput>,
    mut axis_events: EventWriter<MouseWheel>,
    mut keyboard_events: EventWriter<KeyboardInput>,
    keycode_state: Res<ButtonInput<KeyCode>>,
    mut lock_state: ResMut<KeyLockState>,
    mut pointer_state: ResMut<PointerState>,
) {
    if let Err(e) = libinput.libinput.dispatch() {
        error!("libinput error: {e}");
    };
    let Some((default_window_entity, _default_window)) = windows.iter().next() else {
        return;
    };
    let mouse_speed = libinput.mouse_speed;
    let mouse_wheel_speed = libinput.mouse_wheel_speed;
    for event in libinput.libinput.by_ref() {
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
                if let KeyboardEvent::Key(k) = e {
                    let key = k.key();
                    let state = k.key_state();
                    let (key_code, logical_key) = convert_keycode(
                        key,
                        &keycode_state,
                        state,
                        &mut lock_state,
                        &mut k.device(),
                    );
                    keyboard_events.send(KeyboardInput {
                        logical_key,
                        key_code,
                        state: match state {
                            tablet_pad::KeyState::Pressed => ButtonState::Pressed,
                            tablet_pad::KeyState::Released => ButtonState::Released,
                        },
                        window: default_window_entity,
                        repeat: false,
                    });
                };
            }
            input::Event::Pointer(e) => {
                match e {
                    PointerEvent::Motion(m) => {
                        let motion = DVec2::new(m.dx(), m.dy()).as_vec2() * mouse_speed;
                        motion_events.send(MouseMotion { delta: motion });
                        debug!("mouse motion: {}", motion);
                        windows.iter_mut().for_each(|(entity, mut window)| {
                            // TODO 改善边界
                            let pos =
                                pointer_state.position + motion;
                            if let Some(relative) = relative_to_window(&window, pos) {
                                pointer_state.position = pos;
                                pointer_state.window = Some(entity);
                                window.set_cursor_position(Some(relative));
                                window.set_physical_cursor_position(Some(relative.as_dvec2()));
                                move_events.send(CursorMoved {
                                    window:entity,
                                    position:relative,
                                    delta: Some(motion), 
                                });
                                debug!("cursor move: absolute: {pos}; relative: {relative} on {entity:?}");
                            }
                        });
                    }
                    PointerEvent::MotionAbsolute(_m) => todo!(),
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
                        debug!("mouse button: button={button:?}, state={state:?}, window={default_window_entity:?}");
                        button_events.send(MouseButtonInput {
                            button,
                            state,
                            window: default_window_entity,
                        });
                    }
                    PointerEvent::ScrollWheel(m) => {
                        debug!(
                            "PointerEvent::ScrollWheel {:?}",
                            (&m, m.scroll_value_v120(Axis::Vertical))
                        );
                        axis_events.send(MouseWheel {
                            unit: bevy::input::mouse::MouseScrollUnit::Pixel,
                            x: m.scroll_value_v120(Axis::Horizontal) as f32 / 120.0
                                * mouse_wheel_speed,
                            y: -m.scroll_value_v120(Axis::Vertical) as f32 / 120.0
                                * mouse_wheel_speed,
                            window: default_window_entity,
                        });
                    }
                    PointerEvent::ScrollFinger(_m) => {}
                    PointerEvent::ScrollContinuous(_m) => {}
                    _ => {}
                };
            }
            input::Event::Touch(e) => {
                warn!("torch device is not supported");
                match e {
                    TouchEvent::Down(_) => {}
                    TouchEvent::Up(_) => {}
                    TouchEvent::Motion(_) => {}
                    TouchEvent::Cancel(_) => {}
                    TouchEvent::Frame(_) => {}
                    _ => {}
                };
            }
            input::Event::Tablet(_e) => {}
            input::Event::TabletPad(_e) => {}
            input::Event::Gesture(_e) => {}
            input::Event::Switch(_e) => {}
            _ => {}
        }
    }
}

pub struct LibInputPlugin;
impl Plugin for LibInputPlugin {
    fn build(&self, app: &mut App) {
        let libinput = unsafe{
            let cell = app.world_mut().as_unsafe_world_cell();
            let mut seat = cell.get_non_send_resource_mut::<SeatState>().unwrap();
            let mut poller = cell.get_non_send_resource_mut::<Poller>().unwrap();
            LibinputDevice::new(&mut seat, &mut poller).unwrap()
        };
        app.add_systems(First, receive_events.in_set(DWayTTYSet::LibinputSystem))
            .insert_non_send_resource(libinput)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<KeyLockState>()
            .init_resource::<PointerState>()
            .register_type::<KeyLockState>()
            .register_type::<PointerState>()
            .add_event::<MouseMotion>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseWheel>()
            .add_event::<KeyboardInput>();
    }
}
