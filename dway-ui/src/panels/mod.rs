use dway_client_core::navigation::windowstack::{WindowStack};
use dway_server::xdg::toplevel::DWayToplevel;
use dway_ui_framework::{make_bundle, theme::{ThemeComponent, WidgetKind}};

use crate::prelude::*;

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

make_bundle!{
    PanelButtonBundle {
        pub button: UiButtonExt,
        pub material: Handle<RoundedUiRectMaterial>,
    }
}

impl PanelButtonBundle {
    pub fn new(
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
    ) -> Self {
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: UiButtonExt {
                button: UiButton::default(),
                theme: ThemeComponent::widget(WidgetKind::None),
                ..Default::default()
            },
            material: rect_material_set.add(rounded_rect(theme.color("panel"), 8.0)),
            ..Default::default()
        }
    }
    pub fn with_callback(
        theme: &Theme,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        callback: &[(Entity, SystemId<UiButtonEvent>)],
    ) -> Self {
        Self {
            style: Style {
                margin: UiRect::axes(Val::Px(4.0), Val::Auto),
                ..Default::default()
            },
            button: UiButtonExt {
                button: UiButton::with_callbacks(callback),
                theme: ThemeComponent::widget(WidgetKind::None),
                ..Default::default()
            },
            material: rect_material_set.add(rounded_rect(theme.color("panel"), 8.0)),
            ..Default::default()
        }
    }
}
