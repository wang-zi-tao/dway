use smithay::{
    delegate_keyboard_shortcuts_inhibit,
    wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitHandler,
};

use crate::DWay;

impl KeyboardShortcutsInhibitHandler for DWay {
    fn keyboard_shortcuts_inhibit_state(
        &mut self,
    ) -> &mut smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState {
        todo!()
    }

    fn new_inhibitor(
        &mut self,
        inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
    ) {
    }

    fn inhibitor_destroyed(
        &mut self,
        inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
    ) {
    }
}

delegate_keyboard_shortcuts_inhibit!(DWay);
