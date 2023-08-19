pub mod clock;
pub mod cursor;
pub mod window;

use bevy::prelude::*;
use kayak_ui::{prelude::*, KayakUIPlugin};

#[derive(Default)]
pub struct DWayWidgetsPlugin {}
impl Plugin for DWayWidgetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(clock::DWayClockPlugin::default());
    }
}
impl KayakUIPlugin for DWayWidgetsPlugin {
    fn build(&self, context: &mut KayakRootContext) {
        context.add_plugin(clock::DWayClockPlugin::default());
        context.add_plugin(window::DWayWindowPlugin::default());
        context.add_plugin(cursor::CursorPlugin::default());
    }
}
