use std::{
    os::fd::{AsFd, RawFd},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use bevy::{prelude::*, utils::HashMap};
use crossbeam::queue::ArrayQueue;
use getset::Getters;
use libseat::{Seat, SeatEvent};

use crate::schedule::DWayTTYSet;

#[derive(Debug, Component, Clone)]
pub struct DeviceFd {
    pub(crate) device: Arc<libseat::Device>,
}

impl DeviceFd {
    pub fn new(device: libseat::Device) -> Self {
        Self {
            device: Arc::new(device),
        }
    }
}
impl AsFd for DeviceFd {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.device.as_fd()
    }
}

#[derive(Getters)]
#[get = "pub"]
pub struct SeatState {
    pub(crate) name: String,
    pub(crate) seat: Arc<Mutex<Seat>>,
    pub(crate) enable: bool,
    pub(crate) devices: HashMap<RawFd, Entity>,
    pub(crate) queue: Arc<ArrayQueue<SeatEvent>>,
}

impl SeatState {
    #[tracing::instrument(skip_all)]
    pub fn new() -> Result<Self> {
        let queue = Arc::new(ArrayQueue::<SeatEvent>::new(1));
        let tx = queue.clone();
        let mut seat = Seat::open(move |seat, event| {
            debug!(seat = seat.name(), "seat event: {event:?}");
            tx.force_push(event);
        })?;

        seat.dispatch(0).unwrap();
        let active = matches!(queue.pop(), Some(SeatEvent::Enable));

        let name = seat.name();
        info!("new seat: {name:?}");
        Ok(Self {
            name: name.to_string(),
            seat: Arc::new(Mutex::new(seat)),
            enable: active,
            devices: Default::default(),
            queue,
        })
    }

    pub fn open_device(&mut self, path: &PathBuf) -> Result<DeviceFd> {
        let device = self.seat.lock().unwrap().open_device(path)?;
        Ok(DeviceFd::new(device))
    }
}

pub fn process_seat_event(mut seat: NonSendMut<SeatState>) {
    if let Err(e) = seat.seat.lock().unwrap().dispatch(0) {
        error!("seat error: {e}");
    };
    while let Some(event) = seat.queue.clone().pop() {
        match event {
            SeatEvent::Enable => seat.enable = true,
            SeatEvent::Disable => seat.enable = false,
        }
    }
}

pub struct SeatPlugin;
impl Plugin for SeatPlugin {
    fn build(&self, app: &mut App) {
        app.insert_non_send_resource(SeatState::new().unwrap())
            .add_systems(First, process_seat_event.in_set(DWayTTYSet::SeatSystem));
    }
}

#[cfg(test)]
mod tests {
    use super::SeatPlugin;
    use bevy::{log::LogPlugin, prelude::App};

    #[test]
    pub fn test_seat_plugin() {
        App::new()
            .add_plugins((LogPlugin::default(), SeatPlugin))
            .update();
    }
}
