#![feature(iterator_try_collect)]
#![feature(extract_if)]
#![feature(result_flattening)]

use std::time::Duration;

use bevy::{
    app::{ScheduleRunnerPlugin, ScheduleRunnerSettings},
    prelude::*,
};
use drm::DrmPlugin;
use render::TtyRenderPlugin;

pub mod drm;
pub mod egl;
pub mod failure;
pub mod gbm;
pub mod libinput;
pub mod render;
pub mod schedule;
pub mod seat;
pub mod udev;
pub mod util;
pub mod window;

#[derive(Default)]
pub struct DWayTTYPlugin {}

impl Plugin for DWayTTYPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ScheduleRunnerPlugin)
            .add_plugin(seat::SeatPlugin)
            .add_plugin(libinput::LibInputPlugin)
            .add_plugin(udev::UDevPlugin {
                sub_system: "drm".into(),
            })
            .add_plugin(DrmPlugin)
            .add_plugin(TtyRenderPlugin);
    }
}

#[cfg(test)]
mod test {
    use std::fs::OpenOptions;

    use bevy::{log::LogPlugin, prelude::*};

    use crate::DWayTTYPlugin;

    #[test]
    pub fn test_launch() {
        let log_file = OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open("../output/tty.log")
            .unwrap();
        let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);
        tracing_subscriber::fmt().with_writer(non_blocking).init();

        let mut app = App::new();
        app.add_plugin(LogPlugin::default())
            .add_plugin(DWayTTYPlugin::default());
        app.update();
    }
}
