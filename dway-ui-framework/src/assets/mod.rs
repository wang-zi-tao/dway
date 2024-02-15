use crate::prelude::*;
use bevy::asset::embedded_asset;

pub struct UiAssetsPlugin;
impl Plugin for UiAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf.woff2");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf.woff2");
        embedded_asset!(app, "assets", "fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets", "fonts/FiraMono-Medium.ttf");
    }
}
