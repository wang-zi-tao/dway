pub mod device;
pub mod device_manager;
pub mod offer;
pub mod source;

use wayland_protocols_misc::gtk_primary_selection::server::*;

use crate::{prelude::*, state::add_global_dispatch};

pub struct GtkPrimarySelectionPlugin;

impl Plugin for GtkPrimarySelectionPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<
            gtk_primary_selection_device_manager::GtkPrimarySelectionDeviceManager,
            1,
        >(app);
    }
}
