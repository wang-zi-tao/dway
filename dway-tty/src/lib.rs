#![feature(iterator_try_collect)]
#![feature(extract_if)]
#![feature(result_flattening)]

use std::time::Duration;

use bevy::{app::AppExit, prelude::*};
use drm::DrmPlugin;
use dway_util::eventloop::{EventLoop, EventLoopControl, EventLoopPlugin};
use render::TtyRenderPlugin;

pub mod drm;
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
        app.add_plugins((
            EventLoopPlugin::ManualMode,
            seat::SeatPlugin,
            libinput::LibInputPlugin,
            udev::UDevPlugin {
                sub_system: "drm".into(),
            },
            DrmPlugin,
            TtyRenderPlugin,
        ));
        app.set_runner(runner);
    }
}

fn runner(mut app: App) {
    // while !app.ready() {
    //     bevy::tasks::tick_global_task_pools_on_main_thread();
    // }
    app.finish();
    app.cleanup();
    let runner = app.world.non_send_resource_mut::<EventLoop>().runner();
    runner.run(Duration::from_secs(1), move || {
        if !app.world.resource_mut::<Events<AppExit>>().is_empty() {
            return EventLoopControl::Stop;
        }
        app.update();
        EventLoopControl::Continue
    });
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
        app.add_plugins((LogPlugin::default(), DWayTTYPlugin::default()));
        app.update();
    }
}
