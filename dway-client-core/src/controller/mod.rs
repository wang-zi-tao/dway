pub mod bluetooth;
pub mod brightness;
pub mod dbus;
pub mod network;
pub mod notify;
pub mod player;
pub mod systemcontroller;
pub mod systeminfo;
pub mod tray;
pub mod userinfo;
pub mod volume;
pub mod weathre;

use std::time::Duration;

use bevy::time::{TimeSystems, common_conditions::on_timer};
use smart_default::SmartDefault;

use self::{
    dbus::DBusController,
    notify::{NotifyController, NotifyRequest},
    systemcontroller::SystemControllRequest,
    systeminfo::SystemInfo,
    userinfo::UserInfo,
};
use crate::controller::volume::VolumeController;
pub use crate::prelude::*;

#[derive(SmartDefault)]
pub struct ControllerPlugin {
    #[default(Duration::from_secs_f32(1.0))]
    timer: Duration,
}

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<VolumeController>()
            .init_non_send_resource::<DBusController>()
            .init_resource::<SystemInfo>()
            .init_resource::<UserInfo>()
            .init_resource::<NotifyController>()
            .add_event::<SystemControllRequest>()
            .add_event::<NotifyRequest>()
            .add_systems(
                FixedFirst,
                (
                    volume::update_volume_controller,
                    systeminfo::update_system_info_system,
                )
                    .in_set(DWayClientSystem::UpdateSystemInfo)
                    .after(TimeSystems)
                    .run_if(on_timer(self.timer)),
            )
            .add_systems(
                Last,
                (
                    systemcontroller::receive_system_controll_request
                        .run_if(on_event::<SystemControllRequest>),
                    notify::do_receive_notify,
                ),
            );
    }
}
