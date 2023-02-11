#![feature(arc_unwrap_or_clone)]
pub mod background;
pub mod contexts;
pub mod dock;
pub mod lock;
pub mod overview;
pub mod panel;
pub mod title_bar;
pub mod widgets;

use std::sync::Arc;

use failure::Fallible;
pub use kayak_ui;

use bevy::prelude::*;
use font_kit::{
    error::SelectionError,
    family_name::FamilyName,
    handle::Handle,
    properties::{Properties, Style},
    source::SystemSource,
};
use kayak_ui::{prelude::*, widgets::*};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(setup);
        app.add_plugin(panel::DWayPanelPlugin::default());
        app.add_plugin(widgets::DWayWidgetsPlugin::default());
        app.add_plugin(background::DWayBackgroundPlugin::default());
    }
}
pub fn default_system_font() -> Result<Handle, SelectionError> {
    let source = SystemSource::new();
    let default_fonts = &[
        FamilyName::Title("arial".to_string()),
        FamilyName::SansSerif,
        FamilyName::Monospace,
        FamilyName::Fantasy,
    ];
    source.select_best_match(default_fonts, Properties::new().style(Style::Normal))
}

fn setup(
    mut commands: Commands,
    mut font_mapping: ResMut<FontMapping>,
    _font_resource: ResMut<Assets<Font>>,
    asset_server: Res<AssetServer>,
) {
    match default_system_font()
        .map_err(|e| e.into())
        .and_then(|font| font.load().map_err(|e| e.into()))
    {
        Fallible::Ok(font) => {
            let font: font_kit::font::Font = font;
            let font_data_list = font.copy_font_data();
            if let Some(font_data) = font_data_list {
                if let Ok(bevy_font) = Font::try_from_bytes(Arc::unwrap_or_clone(font_data)) {
                    let _e = bevy_font.font;
                    // let kayak_ui_frot=KayakFontLoader.load(font, load_context);
                }
            }
            // font_mapping.set_default();
        }
        Err(e) => {
            error!("failed to load system font :{e}");
            font_mapping.set_default(asset_server.load("roboto.kayak_font"));
        }
    }
    // font_mapping.set_default(asset_server.load("roboto.kayak_font"));
    // let font=asset_server.load("fonts/FiraSans-Bold.ttf");
    font_mapping.set_default(asset_server.load("roboto.kttf"));
    let mut widget_context = KayakRootContext::new();
    widget_context.add_plugin(KayakWidgetsContextPlugin);
    widget_context.add_plugin(panel::DWayPanelPlugin::default());
    widget_context.add_plugin(widgets::DWayWidgetsPlugin::default());
    widget_context.add_plugin(background::DWayBackgroundPlugin::default());
    let parent_id = None;
    // let image = asset_server.load("background.jpg");
    rsx! {
        <KayakAppBundle
        styles={KStyle {
            layout_type:LayoutType::Column.into(),
            position_type: KPositionType::SelfDirected.into(),
            ..Default::default()
        }}
        >
            // <background::DWayBackgroundBundle
            // styles={KStyle {
            //     // left:Units::Pixels(0.0).into(),
            //     // right:Units::Pixels(100.0).into(),
            //     // top:Units::Pixels(100.0).into(),
            //     // bottom:Units::Percentage(100.0).into(),
            //     // z_index: (-1024).into(),
            //     position_type: KPositionType::ParentDirected.into(),
            //         background_color: StyleProp::Value(Color::rgba(1.0, 1.0, 1.0, 0.5)),
            //         color: StyleProp::Value(Color::rgba(0.0, 0.0, 0.0, 1.0)),
            //         layout_type: LayoutType::Row.into(),
            //         // top: StyleProp::Value(Units::Pixels(4.0)),
            //         // left: StyleProp::Value(Units::Pixels(4.0)),
            //         // right: StyleProp::Value(Units::Pixels(4.0)),
            //         padding: StyleProp::Value(Edge::axis( Units::Pixels(2.0) , Units::Pixels(16.0) )),
            //         width: StyleProp::Value(Units::Pixels(1.0)),
            //         height: StyleProp::Value(Units::Pixels(32.0)),
            //         border_radius: StyleProp::Value(Corner::all(100.0)),
            //
            //         ..Default::default()
            //     }}
            // />
                // <KImageBundle
                // image={KImage(image)}
                // styles={KStyle{
                //     left:Units::Pixels(0.0).into(),
                //     right:Units::Pixels(0.0).into(),
                //     top:Units::Pixels(0.0).into(),
                //     bottom:Units::Pixels(0.0).into(),
                // position_type: KPositionType::SelfDirected.into(),
                //     // z_index: (-1024).into(),
                //     ..Default::default()
                // }}
                // />
            <panel::DWayPanelBundle
            styles={KStyle {
                height:Units::Percentage(100.0).into(),
                // z_index: (-1024).into(),
                position_type: KPositionType::SelfDirected.into(),
                ..Default::default()
            }}
            />
        </KayakAppBundle>
    };

    let camera = UICameraBundle::new(widget_context);
    commands.spawn(camera);
}
