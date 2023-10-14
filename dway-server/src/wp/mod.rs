use wayland_protocols::wp::text_input::zv3::server::zwp_text_input_manager_v3;

use crate::prelude::*;

pub mod data_device;
pub mod drmlease;
pub mod primary_selection;
pub mod text_input;

pub struct PrimarySelectionPlugin;
impl Plugin for PrimarySelectionPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            data_device::DataDevicePlugin,
            primary_selection::PrimarySelectionDevicePlugin,
        ));
        app.add_systems(
            PreUpdate,
            create_global_system_config::<zwp_text_input_manager_v3::ZwpTextInputManagerV3, 1>(),
        );
    }
}
