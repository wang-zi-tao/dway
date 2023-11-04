pub mod clock;
pub mod popup;
pub mod window;

pub mod windowlist;
pub mod workspacelist;
pub mod applist;

use bevy::prelude::Plugin;

#[derive(Default)]
pub struct DWayWidgetsPlugin;
impl Plugin for DWayWidgetsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((
            clock::ClockUiPlugin,
            window::WindowUIPlugin,
            popup::PopupUIPlugin,
            applist::AppListUIPlugin,
        ));
    }
}
