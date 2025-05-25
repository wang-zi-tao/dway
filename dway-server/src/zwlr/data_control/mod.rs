pub mod source;
pub mod manager;
pub mod device;
pub mod offer;

use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;

use crate::{prelude::*, state::add_global_dispatch};

pub struct DataControlPlugin;

impl Plugin for DataControlPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<ZwlrDataControlManagerV1, 2>(app);
    }
}
