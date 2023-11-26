pub mod clock;
pub mod popupwindow;
pub mod window;
pub mod popup;

pub mod windowlist;
pub mod workspacelist;
pub mod applist;

use bevy::prelude::Plugin;
use crate::render;

#[derive(Default)]
pub struct DWayWidgetsPlugin;
impl Plugin for DWayWidgetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            clock::ClockUiPlugin,
            window::WindowUIPlugin,
            popupwindow::PopupUIPlugin,
            applist::AppListUIPlugin,
            popup::PopupUiPlugin,
        ));
    }
}
