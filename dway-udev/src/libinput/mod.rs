use std::{
    os::fd::{AsFd, AsRawFd, FromRawFd, IntoRawFd, OwnedFd, RawFd},
    sync::{Arc, Mutex},
};

use anyhow::Result;
use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    utils::HashMap,
};
use input::{
    event::{
        keyboard::KeyboardKeyEvent,
        pointer::{PointerAxisEvent, PointerButtonEvent},
        KeyboardEvent, PointerEvent, TouchEvent,
    },
    Libinput, LibinputInterface,
};
use libseat::Seat;

use crate::{schedule::DWayTTYSet, seat::SeatState};

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
        let libinput = Libinput::new_with_udev(interface);
        Ok(Self { libinput })
    }
}

#[tracing::instrument(skip_all)]
pub fn receive_events(
    mut libinput: NonSendMut<LibinputDevice>,
    mut motion_events: EventWriter<MouseMotion>,
    mut button_events: EventWriter<MouseButtonInput>,
    mut axis_events: EventWriter<MouseWheel>,
    mut keyboard_events: EventWriter<KeyboardInput>,
) {
    while let Some(event) = libinput.libinput.next() {
        match event {
            input::Event::Device(e) => {
                match e {
                    input::event::DeviceEvent::Added(_) => todo!(),
                    input::event::DeviceEvent::Removed(_) => todo!(),
                    _ => todo!(),
                };
            }
            input::Event::Keyboard(e) => {
                match e {
                    KeyboardEvent::Key(_) => todo!(),
                    _ => todo!(),
                };
            }
            input::Event::Pointer(e) => {
                match e {
                    PointerEvent::Motion(_) => todo!(),
                    PointerEvent::MotionAbsolute(_) => todo!(),
                    PointerEvent::Button(_) => todo!(),
                    PointerEvent::Axis(_) => todo!(),
                    PointerEvent::ScrollWheel(_) => todo!(),
                    PointerEvent::ScrollFinger(_) => todo!(),
                    PointerEvent::ScrollContinuous(_) => todo!(),
                    _ => todo!(),
                };
            }
            input::Event::Touch(e) => {
                match e {
                    TouchEvent::Down(_) => todo!(),
                    TouchEvent::Up(_) => todo!(),
                    TouchEvent::Motion(_) => todo!(),
                    TouchEvent::Cancel(_) => todo!(),
                    TouchEvent::Frame(_) => todo!(),
                    _ => todo!(),
                };
            }
            input::Event::Tablet(e) => todo!(),
            input::Event::TabletPad(e) => todo!(),
            input::Event::Gesture(e) => todo!(),
            input::Event::Switch(e) => todo!(),
            _ => {}
        }
        todo!();
    }
}

pub struct LibInputPlugin;
impl Plugin for LibInputPlugin {
    fn build(&self, app: &mut App) {
        let mut seat = app.world.non_send_resource_mut::<SeatState>();
        let libinput = LibinputDevice::new(&mut seat).unwrap();
        app.insert_non_send_resource(libinput)
            .add_system(receive_events.in_set(DWayTTYSet::LibinputSystem))
            .add_event::<MouseMotion>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseWheel>()
            .add_event::<KeyboardInput>();
    }
}
