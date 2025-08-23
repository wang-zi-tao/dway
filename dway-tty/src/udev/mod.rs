use std::{
    collections::VecDeque,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use anyhow::Result;
use bevy::{prelude::*, platform::collections::HashMap};
use dway_util::eventloop::{Poller, PollerGuard};
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

#[derive(Deref)]
pub struct UDevMonitor {
    #[deref]
    pub(crate) monitor: PollerGuard<MonitorSocket>,
    pub(crate) events: VecDeque<UDevEvent>,
    pub device_entity_map: HashMap<PathBuf, Entity>,
}

impl UDevMonitor {
    #[tracing::instrument(skip_all)]
    pub fn new(sub_system: &OsStr, poller: &mut Poller) -> Result<Self> {
        let monitor = MonitorBuilder::new()?
            .match_subsystem(sub_system)?
            .listen()?;
        Ok(Self {
            monitor: poller.add(monitor),
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
        let mut poller = app.world_mut().non_send_resource_mut::<Poller>();
        let udev = UDevMonitor::new(&self.sub_system, &mut poller).unwrap();
        app.insert_non_send_resource(udev);
        app.add_systems(First, receive_events.in_set(DWayTTYSet::UdevSystem));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{log::LogPlugin, prelude::App};
    use dway_util::eventloop::Poller;

    use super::UDevPlugin;

    #[test]
    pub fn test_udev_plugin() {
        App::new()
            .insert_non_send_resource(Poller::new(Duration::from_secs(1)))
            .add_plugins((
                LogPlugin::default(),
                UDevPlugin {
                    sub_system: "drm".into(),
                },
            ))
            .update();
    }
}
