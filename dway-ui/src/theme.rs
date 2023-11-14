use bevy::utils::HashMap;
use bevy_svg::prelude::Svg;

use crate::prelude::*;

pub mod classname {
    pub const BACKGROUND: &str = "background";
    pub const BACKGROUND1: &str = "background1";
    pub const BACKGROUND2: &str = "background2";

    pub const FOREGROUND: &str = "foreground";
    pub const FOREGROUND1: &str = "foreground1";
    pub const FOREGROUND2: &str = "foreground2";

    pub const POPUP_BACKGROUND: &str = "popup-background";
}

pub mod iconname {
    pub const CLOSE: &str = "close";
    pub const MAXIMIZE: &str = "maximize";
    pub const MINIMIZE: &str = "minimize";
}
use classname::*;

#[derive(Resource)]
pub struct Theme {
    pub default_font: Handle<Font>,
    pub color_map: HashMap<String, Color>,
    pub style_map: HashMap<String, Style>,
    pub icons: HashMap<String, Handle<Svg>>,
}

impl Theme {
    pub fn default_font(&self) -> Handle<Font> {
        self.default_font.clone()
    }
    pub fn color(&self, color: &str) -> Option<Color> {
        self.color_map.get(color).cloned()
    }
    pub fn icon(&self, icon: &str) -> Handle<Svg> {
        self.icons.get(icon).cloned().unwrap_or_default()
    }
}

pub struct ThemePlugin;
impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        let asset_server = app.world.resource_mut::<AssetServer>();
        let theme = Theme {
            default_font: asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf"),
            color_map: HashMap::from([
                ("foreground".to_string(), color!("#ffffff")),
                ("foreground1".to_string(), color!("#D8DEE9")),
                ("foreground2".to_string(), color!("#C8CED9")),
                ("background".to_string(), color!("#10171e")),
                ("background".to_string(), color!("#101213")),
                ("background1".to_string(), color!("#131a21")),
                ("background2".to_string(), color!("#1a222a")),
                ("background1".to_string(), color!("#1b1d1e")),
                ("background2".to_string(), color!("#2b2d2e")),
                ("background3".to_string(), color!("#3b3d3e")),
                ("black".to_string(), color!("#1C252C")),
                ("red".to_string(), color!("#DF5B61")),
                ("green".to_string(), color!("#78B892")),
                ("orange".to_string(), color!("#DE8F78")),
                ("blue".to_string(), color!("#6791C9")),
                ("purple".to_string(), color!("#BC83E3")),
                ("sky".to_string(), color!("#67AFC1")),
                ("white".to_string(), color!("#D9D7D6")),
                ("gray".to_string(), color!("#484E5B")),
                (POPUP_BACKGROUND.to_string(), color!("#D8DEE9")),
            ]),
            icons: HashMap::from([
                (
                    "close".to_string(),
                    asset_server.load("embedded://dway_ui/icons/close.svg"),
                ),
                (
                    "maximize".to_string(),
                    asset_server.load("embedded://dway_ui/icons/maximize.svg"),
                ),
                (
                    "minimize".to_string(),
                    asset_server.load("embedded://dway_ui/icons/minimize.svg"),
                ),
            ]),
            style_map: HashMap::from([("popup".to_string(), style!("m-4"))]),
        };
        app.insert_resource(theme);
    }
}
