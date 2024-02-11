use crate::prelude::*;
use bevy::asset::embedded_asset;

pub struct DWayAssetsPlugin;
impl Plugin for DWayAssetsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "assets", "icons/apps.svg");
        embedded_asset!(app, "assets", "icons/close.svg");
        embedded_asset!(app, "assets", "icons/dashboard.svg");
        embedded_asset!(app, "assets", "icons/lock.svg");
        embedded_asset!(app, "assets", "icons/logout.svg");
        embedded_asset!(app, "assets", "icons/maximize.svg");
        embedded_asset!(app, "assets", "icons/minimize.svg");
        embedded_asset!(app, "assets", "icons/power.svg");
        embedded_asset!(app, "assets", "icons/restart.svg");
        embedded_asset!(app, "assets", "icons/settings.svg");
        embedded_asset!(app, "assets", "icons/user.svg");
        embedded_asset!(app, "assets", "icons/volume_off.svg");
        embedded_asset!(app, "assets", "icons/volume_on.svg");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf.woff2");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf.woff2");
        embedded_asset!(app, "assets", "fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets", "fonts/FiraMono-Medium.ttf");
    }
}
