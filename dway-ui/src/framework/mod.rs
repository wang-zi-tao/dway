pub mod animation;
pub mod button;
pub mod canvas;
pub mod drag;
pub mod evnet;
pub mod gallary;
pub mod icon;
pub mod slider;
pub mod svg;
pub mod text;

use crate::prelude::*;
use bevy::input::mouse::MouseMotion;
pub use bevy_svg::SvgPlugin;
use dway_util::UtilPlugin;
use smart_default::SmartDefault;

use self::{button::ButtonColor, drag::Draggable};

#[derive(Component, Default)]
pub struct Callback(pub Option<SystemId>);

#[derive(Bundle, SmartDefault)]
pub struct MiniNodeBundle {
    pub node: Node,
    pub style: Style,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

#[derive(Bundle, SmartDefault)]
pub struct MiniButtonBundle {
    pub node: Node,
    pub style: Style,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,

    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
    pub interaction: Interaction,
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
                (button::process_ui_button_event,),
                svg::uisvg_render.after(canvas::UiCanvasSystems::Prepare),
                drag::update_draggable
                    .run_if(on_event::<MouseMotion>().and_then(any_with_component::<Draggable>())),
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
        app.register_type::<button::UiButton>();
        app.register_type::<button::ButtonColor>();
        app.register_type::<drag::Draggable>();
        app.init_resource::<canvas::UiCanvasRenderArea>();
        app.init_resource::<svg::SvgImageCache>();
        app.register_system(ButtonColor::callback_system::<RoundedUiRectMaterial>);
        app.register_system(ButtonColor::callback_system::<UiCircleMaterial>);
        app.add_plugins((slider::UiSliderPlugin, gallary::WidgetGallaryPlugin));
    }
}
