use bevy::ui::RelativeCursorPosition;
use smart_default::SmartDefault;

use crate::{
    prelude::*,
    theme::{StyleFlags, ThemeComponent, WidgetKind},
};

#[derive(Component)]
pub struct UiSliderInited;

#[derive(Component, SmartDefault, Reflect)]
#[require(RelativeCursorPosition, Interaction, UiSliderEventDispatcher, ThemeComponent)]
pub struct UiSlider {
    #[default(1.0)]
    pub max: f32,
    #[default(0.0)]
    pub min: f32,
}

#[derive(Debug, Clone)]
pub enum UiSliderEventKind {
    ValueChanged(f32),
}

#[derive(Clone, Debug)]
pub struct UiSliderEvent {
    pub value: f32,
    pub kind: UiSliderEventKind,
}

pub type UiSliderEventDispatcher = EventDispatcher<UiSliderEvent>;

dway_widget! {
UiSlider=>
@plugin{app.register_type::<UiSlider>();}
@state_reflect()
@use_state(pub value: f32)
@world_query(slider_interaction: Ref<Interaction>)
@world_query(relative_cursor_position: Ref<RelativeCursorPosition>)
@world_query(event_dispatcher: Ref<UiSliderEventDispatcher>)
@world_query(node: &mut Node)
@world_query(computed_node: &ComputedNode)
@before{
if !widget.inited{
    commands.entity(this_entity).insert(UiSliderInited);
}
if ( slider_interaction.is_changed() || relative_cursor_position.is_changed() )
        && *slider_interaction == Interaction::Pressed{
    if let Some(mouse_position) = get_node_mouse_position(&relative_cursor_position, computed_node) {
        let raw_value = mouse_position.x.max(0.0).min(1.0);
        state.set_value(raw_value*(prop.max-prop.min)+prop.min);
        event_dispatcher.send(UiSliderEvent{
            value: *state.value(),
            kind: UiSliderEventKind::ValueChanged(*state.value()),
        }, commands);
    }
} }
<Node @id="bar" @style="absolute h-8 w-full min-h-8 align-self:center" >
    <(Node{
        width: Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0)),
        ..style!("m-2")
    }) @id="bar_highlight"
/>
</Node>
<(Node{
    margin:UiRect::left(Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0))),
    ..style!("absolute w-0 h-full flex-col align-items:center justify-content:center align-self:center")
}) >
    <Node @id="handle" @style="absolute align-self:center w/h-1.0 h-80% min-w-16 min-h-16" />
</Node>
}
