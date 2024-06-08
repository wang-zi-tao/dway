#![feature(iterator_try_collect)]
#![feature(extract_if)]
#![feature(result_flattening)]

use std::time::{Duration, Instant};

use bevy::{
    app::{AppExit, PluginsState},
    core::FrameCount,
    ecs::event::ManualEventReader,
    prelude::*,
    window::RequestRedraw,
};
use drm::DrmPlugin;
use dway_util::eventloop::{EventLoop, EventLoopControl, EventLoopPlugin, EventLoopPluginMode};
use measure_time::debug_time;
use render::TtyRenderPlugin;
use schedule::DWayTtySchedulePlugin;
use smart_default::SmartDefault;

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

#[derive(Resource, Debug, SmartDefault)]
pub struct DWayTTYSettings {
    #[default(Duration::from_secs_f32(1.0/144.0))]
    pub frame_duration: Duration,
}

#[derive(Default)]
pub struct DWayTTYPlugin {}

impl Plugin for DWayTTYPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DWayTtySchedulePlugin,
            EventLoopPlugin {
                mode: EventLoopPluginMode::ManualMode,
            },
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
    let plugins_state = app.plugins_state();
    if plugins_state != PluginsState::Cleaned {
        while app.plugins_state() == PluginsState::Adding {
            #[cfg(not(target_arch = "wasm32"))]
            bevy::tasks::tick_global_task_pools_on_main_thread();
        }
        app.finish();
        app.cleanup();
    }

    let mut redraw_events_reader = ManualEventReader::<RequestRedraw>::default();

    let runner = app.world.non_send_resource_mut::<EventLoop>().runner();
    runner.run(Duration::from_secs_f32(0.2), move || {
        let start_time = Instant::now();
        if !app.world.resource_mut::<Events<AppExit>>().is_empty() {
            return EventLoopControl::Stop;
        }
        let frame = app.world.resource::<FrameCount>();
        debug!("frame number: {}", frame.0);
        debug_time!("frame {}", frame.0);
        app.update();
        let end_time = Instant::now();

        if let Some(frame) = app.world.get_resource::<DWayTTYSettings>() {
            if end_time - start_time < frame.frame_duration {
                std::thread::sleep(frame.frame_duration - (end_time - start_time));
            }

            if redraw_events_reader
                .read(app.world.resource())
                .last()
                .is_some()
            {
                return EventLoopControl::ContinueImmediate;
            }
        };

        EventLoopControl::Continue
    });
}

#[cfg(test)]
mod test {

    use bevy::{
        app::{AppExit, ScheduleRunnerPlugin},
        core::FrameCount,
        log::LogPlugin,
        prelude::*,
        winit::WinitPlugin,
    };
    use dway_util::logger::DWayLogPlugin;
    use tracing::Level;

    use crate::DWayTTYPlugin;

    #[test]
    pub fn test_launch() {
        let mut app = App::new();
        app.add_plugins(test_suite_plugins())
            .add_plugins(DWayTTYPlugin::default());
        app.add_systems(
            Last,
            |frame: Res<FrameCount>, mut exit: EventWriter<AppExit>| {
                if frame.0 > 2 {
                    exit.send_default();
                }
            },
        );
        app.run();
    }

    pub fn test_suite_plugins() -> bevy::app::PluginGroupBuilder {
        DefaultPlugins
            .build()
            .disable::<LogPlugin>()
            .disable::<WinitPlugin>()
            .add(ScheduleRunnerPlugin::run_once())
            .add(DWayLogPlugin {
                filter: Default::default(),
                level: Level::TRACE,
            })
    }
}
