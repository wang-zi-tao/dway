use smithay::{delegate_primary_selection, wayland::primary_selection::PrimarySelectionHandler};

use crate::DWay;

impl PrimarySelectionHandler for DWay {
    fn primary_selection_state(
        &self,
    ) -> &smithay::wayland::primary_selection::PrimarySelectionState {
        todo!()
    }

    fn send_selection(&mut self, mime_type: String, fd: std::os::fd::OwnedFd) {}
}
delegate_primary_selection!(DWay);
