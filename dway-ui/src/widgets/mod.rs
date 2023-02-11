pub mod clock;


use bevy::prelude::*;
use kayak_ui::{prelude::*, KayakUIPlugin};


#[derive(Default)]
pub struct DWayWidgetsPlugin{

}
impl Plugin for DWayWidgetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugin(clock::DWayClockPlugin::default());
    }
}
impl KayakUIPlugin for DWayWidgetsPlugin{
    fn build(&self, context: &mut KayakRootContext) {
        context.add_plugin(clock::DWayClockPlugin::default());
    }
}
