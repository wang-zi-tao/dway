use dway_client_core::navigation::windowstack::{WindowIndex, WindowStack};
use dway_server::xdg::toplevel::DWayToplevel;

use crate::{
    framework::button::{ButtonColor, RoundedButtonAddonBundle, UiButton, UiButtonEvent},
    prelude::*,
};

#[derive(Component, Debug, Default)]
pub struct WindowTitle;

dway_widget! {
WindowTitle=>
@global(stack: WindowStack -> { state.set_window_entity(stack.focused().unwrap_or(Entity::PLACEHOLDER)); })
@use_state(pub window_entity:Entity=Entity::PLACEHOLDER)
@query(window_query:(toplevel)<-Query<Ref<DWayToplevel>>[*state.window_entity()]->{
    if !widget.inited || toplevel.is_changed(){
        state.set_title(toplevel.title.clone().unwrap_or_default());
    }
})
@use_state(pub title: String)
@global(theme: Theme)
<MiniNodeBundle>
    <TextBundle Text=(Text::from_section( state.title(),
        TextStyle { font_size: 24.0, color: theme.color("foreground"), font: theme.default_font() },))/>
</MiniNodeBundle>
}

#[derive(Bundle, Default)]
pub struct PanelButtonBundle {
    pub node: Node,
    pub style: Style,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,

    pub button: RoundedButtonAddonBundle,
}

impl PanelButtonBundle {
    pub fn new(
        entity: Entity,
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
    ) -> Self {
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: RoundedButtonAddonBundle {
                button: UiButton::from_slice(&[(
                    entity,
                    theme.system(ButtonColor::callback_system::<RoundedUiRectMaterial>),
                )]),
                color: ButtonColor::from_theme(&theme, "panel"),
                material: rect_material_set
                    .add(RoundedUiRectMaterial::new(theme.color("panel"), 8.0)),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    pub fn with_callback(
        entity: Entity,
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        callback: &[(Entity, SystemId<UiButtonEvent>)],
    ) -> Self {
        let mut callbacks = callback.to_vec();
        callbacks.push((
            entity,
            theme.system(ButtonColor::callback_system::<RoundedUiRectMaterial>),
        ));
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: RoundedButtonAddonBundle {
                button: UiButton::from_slice(&callbacks),
                color: ButtonColor::from_theme(&theme, "panel"),
                material: rect_material_set
                    .add(RoundedUiRectMaterial::new(theme.color("panel"), 8.0)),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
