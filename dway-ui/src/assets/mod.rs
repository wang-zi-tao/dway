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
        embedded_asset!(app, "assets", "icons/notifications.svg");

        embedded_asset!(app, "assets", "cursors/cursor-default.png");

        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.otf.woff2");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf");
        embedded_asset!(app, "assets", "fonts/SmileySans-Oblique.ttf.woff2");
        embedded_asset!(app, "assets", "fonts/FiraSans-Bold.ttf");
        embedded_asset!(app, "assets", "fonts/FiraMono-Medium.ttf");

        let asset_server = app.world().resource::<AssetServer>();
        let default_font = asset_server.load("embedded://dway_ui/fonts/SmileySans-Oblique.ttf");

        let mut theme = app.world_mut().resource_mut::<Theme>();
        theme.default_font = default_font;
        theme.register_icons_in_dictory(
            "embedded://dway_ui/icons",
            &[
                "apps",
                "close",
                "dashboard",
                "lock",
                "logout",
                "maximize",
                "minimize",
                "power",
                "restart",
                "settings",
                "user",
                "volume_off",
                "volume_on",
                "notifications",
            ],
        );
    }
}
