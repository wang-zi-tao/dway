use smithay::{delegate_primary_selection, wayland::primary_selection::PrimarySelectionHandler};

use crate::DWay;

impl PrimarySelectionHandler for DWay {
    fn primary_selection_state(
        &self,
    ) -> &smithay::wayland::primary_selection::PrimarySelectionState {
        &self.primary_selection_state
    }
}
delegate_primary_selection!(DWay);
