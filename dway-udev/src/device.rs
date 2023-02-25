use std::{cell::RefCell, os::fd::FromRawFd, path::PathBuf, rc::Rc, sync::Mutex};

use bevy::{prelude::*, utils::HashMap};
use dway_server::log::logger;
use failure::Fallible;
use nix::{fcntl::OFlag, sys::stat::stat};
use smithay::{
    backend::{
        drm::{DrmDevice, DrmDeviceFd, DrmNode, NodeType},
        session::{libseat::LibSeatSession, Session},
        udev::{all_gpus, primary_gpu},
    },
    reexports::gbm,
    utils::DeviceFd,
};

use crate::seat::{SeatSession, SeatSessions};

pub struct DeviceSet {
    devices: HashMap<PathBuf, Entity>,
}

#[derive(Component)]
pub struct Device {
    pub path: PathBuf,
    pub drm: DrmDevice,
    pub gbm: Mutex<gbm::Device<DrmDeviceFd>>,
}

pub fn scan_devices(
    seat_set: NonSendMut<SeatSessions>,
    mut device_set: Mut<DeviceSet>,
    mut seates: Query<&mut SeatSession>,
    mut commands: Commands,
) {
    for seat in seates.iter_mut() {
        let mut raw_seat = seat.raw.lock().unwrap();
        let all_gpus = match all_gpus(&raw_seat.seat()) {
            Ok(o) => o,
            Err(e) => {
                warn!("failed to scan gpus, Error: {e}");
                continue;
            }
        };
        for (_, entity) in device_set
            .devices
            .drain_filter(|path, _| !all_gpus.contains(path))
        {
            commands.entity(entity).despawn();
        }
        for gpu_path in all_gpus.iter() {
            if !device_set.devices.contains_key(gpu_path) {
                match scan_gpu(gpu_path, &mut raw_seat) {
                    Ok(o) => {
                        let entity = commands.spawn(o).id();
                        device_set.devices.insert(gpu_path.clone(), entity);
                    }
                    Err(e) => {
                        error!(
                            "Unable connect gpu {:?}, Error: {:?}. Skipping",
                            gpu_path, e
                        );
                        continue;
                    }
                }
            }
        }
    }
}
pub fn scan_gpu(gpu_path: &PathBuf, raw_seat: &mut LibSeatSession) -> Fallible<Device> {
    // let stat = stat(gpu_path)?;
    // let raw_dev = stat.st_rdev;
    // let node = DrmNode::from_dev_id(raw_dev)?;
    let open_flags = OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_NONBLOCK;
    let fd = raw_seat.open(&gpu_path, open_flags)?;
    let logger = logger();
    let drm_fd = DrmDeviceFd::new(unsafe { DeviceFd::from_raw_fd(fd) }, Some(logger.clone()));
    let drm = DrmDevice::new(drm_fd.clone(), true, logger.clone())?;
    let gbm = gbm::Device::new(drm_fd)?;
    Ok(Device {
        path: gpu_path.clone(),
        drm,
        gbm: Mutex::new(gbm),
    })
}
