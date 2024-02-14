pub mod animation;
pub mod button;
pub mod canvas;
pub mod checkbox;
pub mod drag;
pub mod evnet;
pub mod gallary;
pub mod icon;
pub mod scroll;
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

#[derive(Resource, Default, Reflect)]
pub struct MousePosition {
    pub window: Option<Entity>,
    pub position: Option<Vec2>,
}

pub fn update_mouse_position(
    mut mouse_event: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(mouse) = mouse_event.read().last() {
        mouse_position.window = Some(mouse.window);
        mouse_position.position = Some(mouse.position);
    }
}

pub struct UiFrameworkPlugin;
impl Plugin for UiFrameworkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, canvas::cleanup_render_command);
        app.add_systems(
            Update,
            (
                // (icon::uiicon_render, apply_deferred)
                //     .chain()
                //     .before(canvas::prepare_render_command),
                (canvas::prepare_render_command, apply_deferred)
                    .chain()
                    .in_set(canvas::UiCanvasSystems::Prepare),
                (button::process_ui_button_event,),
                (
                    svg::update_uisvg.before(canvas::UiCanvasSystems::Prepare),
                    svg::uisvg_render.after(canvas::UiCanvasSystems::Prepare),
                ),
                drag::update_draggable
                    .run_if(on_event::<MouseMotion>().and_then(any_with_component::<Draggable>)),
                (checkbox::process_ui_checkbox_event),
            ),
        );
        // app.add_systems(
        //     PostUpdate,
        //     animation::after_animation_finish
        //         .run_if(on_event::<TweenCompleted>())
        //         .in_set(animation::AnimationSystems::Finish),
        // );
        // app.add_systems(
        //     Last,
        //     animation::request_update_system.in_set(animation::AnimationSystems::PrepareNextFrame),
        // );
        app.add_systems(
            PreUpdate,
            update_mouse_position.run_if(on_event::<CursorMoved>()),
        );
        app.add_plugins(UtilPlugin);
        app.register_type::<canvas::UiCanvas>();
        app.register_type::<svg::UiSvg>();
        app.register_type::<icon::UiIcon>();
        app.register_type::<button::UiButton>();
        app.register_type::<button::ButtonColor>();
        app.register_type::<drag::Draggable>();
        app.register_type::<MousePosition>();
        app.register_type::<checkbox::UiCheckBox>();
        app.register_type::<checkbox::UiCheckBoxState>();
        app.init_resource::<canvas::UiCanvasRenderArea>();
        app.init_resource::<svg::SvgImageCache>();
        app.init_resource::<MousePosition>();
        // app.register_system(ButtonColor::callback_system::<RoundedUiRectMaterial>);
        // app.register_system(ButtonColor::callback_system::<UiCircleMaterial>);
        // app.register_system(checkbox::checkbox_color_callback::<RoundedUiRectMaterial>);
        // app.register_system(checkbox::checkbox_color_callback::<UiCircleMaterial>);
        app.add_plugins((
            slider::UiSliderPlugin,
            gallary::WidgetGallaryPlugin,
            scroll::UiScrollPlugin,
        ));
    }
}
