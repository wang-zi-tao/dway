use anyhow::Result;
use bevy::{prelude::*, utils::HashMap};
use dway_util::eventloop::EventLoop;
use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    os::fd::AsRawFd,
    path::PathBuf,
};
use udev::{Device, MonitorBuilder, MonitorSocket};

use crate::schedule::DWayTTYSet;

#[derive(Debug)]
pub struct UDevDeviceId(pub udev::Device);

#[derive(Clone, Debug)]
pub enum UDevEvent {
    Added(Device),
    Changed(Device),
    Removed(Device),
}

pub struct UDevMonitor {
    pub(crate) monitor: MonitorSocket,
    pub(crate) events: VecDeque<UDevEvent>,
    pub device_entity_map: HashMap<PathBuf, Entity>,
}

impl AsRawFd for UDevMonitor {
    fn as_raw_fd(&self) -> std::os::fd::RawFd {
        self.monitor.as_raw_fd()
    }
}

impl UDevMonitor {
    #[tracing::instrument(skip_all)]
    pub fn new(sub_system: &OsStr) -> Result<Self> {
        let monitor = MonitorBuilder::new()?
            .match_subsystem(sub_system)?
            .listen()?;
        Ok(Self {
            monitor,
            events: Default::default(),
            device_entity_map: Default::default(),
        })
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &UDevEvent> {
        self.events.iter()
    }

    pub fn receive_events(&mut self) {
        for event in self.monitor.iter() {
            trace!("udev receive event: {event:?}");
            match event.event_type() {
                udev::EventType::Add => {
                    self.events.push_back(UDevEvent::Added(event.device()));
                }
                udev::EventType::Change => {
                    self.events.push_back(UDevEvent::Changed(event.device()))
                }
                udev::EventType::Remove => {
                    self.events.push_back(UDevEvent::Removed(event.device()))
                }
                _ => {}
            }
        }
    }
}

#[tracing::instrument(skip_all)]
pub fn receive_events(mut udev: NonSendMut<UDevMonitor>) {
    udev.clear_events();
    udev.receive_events();
}

pub struct UDevPlugin {
    pub sub_system: OsString,
}
impl Plugin for UDevPlugin {
    fn build(&self, app: &mut App) {
        let udev = UDevMonitor::new(&self.sub_system).unwrap();
        app.world
            .non_send_resource_mut::<EventLoop>()
            .add_fd_to_read(&udev);
        app.insert_non_send_resource(udev);
        app.add_systems(First, receive_events.in_set(DWayTTYSet::UdevSystem));
    }
}

#[cfg(test)]
mod tests {
    use super::UDevPlugin;
    use bevy::{log::LogPlugin, prelude::App};

    #[test]
    pub fn test_udev_plugin() {
        App::new()
            .add_plugins((
                LogPlugin::default(),
                UDevPlugin {
                    sub_system: "drm".into(),
                },
            ))
            .update();
    }
}
