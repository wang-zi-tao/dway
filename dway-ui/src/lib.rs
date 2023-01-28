pub mod dock;
pub mod lock;
pub mod overview;
pub mod panel;
pub mod title_bar;

pub use kayak_ui;

use bevy::prelude::*;
use kayak_ui::{prelude::*, widgets::{*}, KayakUIPlugin};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup);
        app.add_plugin(panel::DWayPanelPlugin::default());
    }
}

fn setup(
    mut commands: Commands,
    mut font_mapping: ResMut<FontMapping>,
    asset_server: Res<AssetServer>,
) {
    font_mapping.set_default(asset_server.load("roboto.kayak_font"));
    let mut widget_context = KayakRootContext::new();
    widget_context.add_plugin(KayakWidgetsContextPlugin);
    widget_context.add_plugin(panel::DWayPanelPlugin::default());
    let parent_id = None;
    rsx! {
        <KayakAppBundle>
            <panel::DWayPanelBundle/>
        </KayakAppBundle>
    };

    commands.spawn(UICameraBundle::new(widget_context));
}
