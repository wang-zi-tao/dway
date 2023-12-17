use std::time::Duration;

use crate::prelude::*;
use bevy::time::common_conditions::on_timer;
use libpulse_binding::volume::Volume;
use pulsectl::controllers::{DeviceControl, SinkController};

pub struct VolumeController {
    pub controller: SinkController,
    pub volume: u32,
    pub mute: bool,
}

impl VolumeController {
    pub fn volume(&self) -> f32 {
        self.volume as f32 / Volume::NORMAL.0 as f32
    }

    pub fn set_mute(&mut self, mute: bool) -> anyhow::Result<()> {
        trace!("set mute: {mute}");
        let device = self.controller.get_default_device()?;
        if self.mute != mute {
            self.controller.set_device_mute_by_index(device.index, mute);
            self.mute = mute;
        }
        Ok(())
    }

    pub fn increase(&mut self, delta: f32) -> anyhow::Result<()> {
        trace!("increase volume: {delta}");
        let device = self.controller.get_default_device()?;
        self.controller
            .increase_device_volume_by_percent(device.index, delta as f64);
        let volume = self.volume + (delta * Volume::NORMAL.0 as f32) as u32;
        if self.volume != volume {
            self.volume = volume;
        }
        Ok(())
    }

    pub fn set_volume(&mut self, value: f32) -> anyhow::Result<()> {
        trace!("set volume: {value}");
        let device = self.controller.get_default_device()?;
        let current_volume = device
            .volume
            .get()
            .first()
            .cloned()
            .map(|v| v.0 as f64)
            .unwrap_or_default();
        self.controller.increase_device_volume_by_percent(
            device.index,
            value as f64 - current_volume / Volume::NORMAL.0 as f64,
        );
        let volume = value as f64 * Volume::NORMAL.0 as f64;
        if self.volume != volume as u32 {
            self.volume = volume as u32;
        }
        Ok(())
    }

    pub fn is_mute(&self) -> bool {
        self.mute
    }
}

impl Default for VolumeController {
    fn default() -> Self {
        let mut controller = SinkController::create().unwrap();
        let device = controller.get_default_device().ok();
        Self {
            volume: device
                .as_ref()
                .and_then(|d| d.volume.get().first().cloned())
                .map(|v| v.0)
                .unwrap_or_default(),
            mute: device.map(|d| d.mute).unwrap_or_default(),
            controller,
        }
    }
}

pub fn update_volume_controller(mut volume_controller: NonSendMut<VolumeController>) {
    if let Ok(device) = volume_controller.controller.get_default_device() {
        let mute = device.mute;
        if volume_controller.mute != mute {
            volume_controller.mute = mute;
        }
        let volume = device
            .volume
            .get()
            .first()
            .cloned()
            .map(|v| v.0)
            .unwrap_or_default();
        if volume_controller.volume != volume {
            volume_controller.volume = volume;
        }
    }
}

pub struct VolumeControllerPlugin;
impl Plugin for VolumeControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<VolumeController>()
            .add_systems(
                First,
                update_volume_controller.run_if(on_timer(Duration::from_secs_f32(0.1))),
            );
    }
}
