pub mod animation;
pub mod button;
pub mod canvas;
pub mod evnet;
pub mod icon;
pub mod svg;

use crate::prelude::*;
pub use bevy_svg::SvgPlugin;
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
        app.add_systems(
            PostUpdate,
            animation::after_animation_finish
                .run_if(on_event::<TweenCompleted>())
                .in_set(animation::AnimationSystems::Finish),
        );
        app.add_plugins(UtilPlugin);
        app.register_type::<canvas::UiCanvas>();
        app.register_type::<svg::UiSvg>();
        app.register_type::<icon::UiIcon>();
        app.init_resource::<canvas::UiCanvasRenderArea>();
        app.init_resource::<svg::SvgImageCache>();
    }
}
