pub mod clock;
pub mod cursor;
pub mod window;
pub mod icon;
pub mod app_entry;
pub mod app_entry_list;

use bevy::prelude::*;
use kayak_ui::{prelude::*, KayakUIPlugin};

#[derive(Default)]
pub struct DWayWidgetsPlugin {}
impl Plugin for DWayWidgetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(clock::DWayClockPlugin::default());
    }
}
impl KayakUIPlugin for DWayWidgetsPlugin {
    fn build(&self, context: &mut KayakRootContext) {
        context.add_plugin(clock::DWayClockPlugin::default());
        context.add_plugin(window::DWayWindowPlugin::default());
        context.add_plugin(cursor::CursorPlugin);
        context.add_plugin(icon::IconPlugin);
        context.add_plugin(app_entry::AppEntryPlugin);
        context.add_plugin(app_entry_list::AppEntryListPlugin);
    }
}
