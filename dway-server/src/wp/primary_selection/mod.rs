pub mod device;
pub mod manager;
pub mod offer;
pub mod source;

use wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_device_manager_v1;

use crate::{prelude::*, state::add_global_dispatch};

relationship!(SourceOfSelection=>SourceRef>-Selection);

pub struct PrimarySelectionPlugin;
impl Plugin for PrimarySelectionPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<
            zwp_primary_selection_device_manager_v1::ZwpPrimarySelectionDeviceManagerV1,
            1,
        >(app);
        app.register_relation::<SourceOfSelection>();
    }
}
