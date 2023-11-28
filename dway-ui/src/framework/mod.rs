pub mod button;
pub mod canvas;
pub mod evnet;
pub mod icon;
pub mod svg;

use bevy::{ecs::system::SystemId, prelude::*, ui::FocusPolicy};
pub use bevy_svg::SvgPlugin;
use bevy_vector_shapes::{Shape2dPlugin, ShapePlugin};
use dway_util::UtilPlugin;

#[derive(Component, Default)]
pub struct Callback(pub Option<SystemId>);

#[derive(Bundle, Default)]
pub struct MiniNodeBundle {
    pub node: Node,
    pub style: Style,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, canvas::cleanup_render_command);
        app.add_systems(
            Update,
            (
                (icon::uiicon_render, apply_deferred)
                    .chain()
                    .before(canvas::prepare_render_command),
                (canvas::prepare_render_command, apply_deferred)
                    .chain()
                    .in_set(canvas::UiCanvasSystems::Prepare),
                (button::process_ui_button_event),
                svg::uisvg_render.after(canvas::UiCanvasSystems::Prepare),
            ),
        );
        if !app.is_plugin_added::<SvgPlugin>() {
            app.add_plugins(SvgPlugin);
        }
        if !app.is_plugin_added::<ShapePlugin>() {
            app.add_plugins(Shape2dPlugin::default());
        }
        app.add_plugins(UtilPlugin);
        app.register_type::<canvas::UiCanvas>();
        app.register_type::<svg::UiSvg>();
        app.register_type::<icon::UiIcon>();
        app.init_resource::<canvas::UiCanvasRenderArea>();
        app.init_resource::<svg::SvgImageCache>();
    }
}
