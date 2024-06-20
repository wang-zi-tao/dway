use dway_client_core::{desktop::FocusedWindow, workspace::WindowList};
use dway_server::{
    geometry::GlobalGeometry, util::rect::IRect, wl::surface::WlSurface,
    xdg::toplevel::DWayToplevel,
};
use dway_ui_framework::widgets::button::{UiRawButtonBundle, UiRawButtonExt};

use crate::{prelude::*, widgets::window::create_raw_window_material};

#[derive(Component, Reflect)]
pub struct WorkspaceWindowPreviewPopup {
    pub workspace: Entity,
    pub scale: f32,
}
impl Default for WorkspaceWindowPreviewPopup {
    fn default() -> Self {
        Self {
            workspace: Entity::PLACEHOLDER,
            scale: 1.0 / 16.0,
        }
    }
}

dway_widget! {
WorkspaceWindowPreviewPopup=>
@global(theme: Theme)
@arg(asset_server: Res<AssetServer>)
<MiniNodeBundle @style="flex-row m-4" @id="List" >
</MiniNodeBundle>
}
