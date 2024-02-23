use bevy::ui::RelativeCursorPosition;
use smart_default::SmartDefault;
use crate::{prelude::*, theme::{StyleFlags, ThemeComponent, WidgetKind}};

#[derive(Component, SmartDefault, Reflect)]
pub struct UiSlider {
    #[reflect(ignore)]
    pub callback: Option<(Entity, SystemId<UiSliderEvent>)>,
    #[default(1.0)]
    pub max: f32,
    #[default(0.0)]
    pub min: f32,
}

#[derive(Debug, Clone)]
pub enum UiSliderEventKind {
    Value(f32),
}

#[derive(Event, Debug)]
pub struct UiSliderEvent {
    pub receiver: Entity,
    pub slider: Entity,
    pub value: f32,
    pub kind: UiSliderEventKind,
}

dway_widget! {
UiSlider=>
@plugin{app.register_type::<UiSlider>();}
@global(theme:Theme)
@state_reflect()
@use_state(pub value: f32)
@bundle({
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy = FocusPolicy::Block,
    pub style: Style = style!("items-center absolute full min-h-16 min-w-32"),
    pub cursor_positon: RelativeCursorPosition, // TODO 优化
})
@world_query(slider_interaction: Ref<Interaction>)
@world_query(mouse_position: Ref<RelativeCursorPosition>)
@before{
if ( slider_interaction.is_changed() || mouse_position.is_changed() )
        && *slider_interaction == Interaction::Pressed{
    if let Some(relative) = mouse_position.normalized {
        let slider_rect = mouse_position.normalized_visible_node_rect;
        let raw_value = (relative.x/slider_rect.size().x).max(0.0).min(1.0);
        state.set_value(raw_value*(prop.max-prop.min)+prop.min);
        if let Some((receiver,callback)) = &prop.callback {
            commands.run_system_with_input(
                *callback,
                UiSliderEvent{
                    receiver: *receiver,
                    slider: this_entity,
                    value: *state.value(),
                    kind: UiSliderEventKind::Value(*state.value()),
                }
            );
        }
    }
} }
<MiniNodeBundle @id="bar" @style="absolute h-8 w-full min-h-8 align-self:center"
    ThemeComponent=(ThemeComponent::new(StyleFlags::default(), WidgetKind::Slider))
>
    <MiniNodeBundle @id="bar_highlight"
        ThemeComponent=(ThemeComponent::new(StyleFlags::default(), WidgetKind::SliderHightlightBar))
        Style=(Style{
        width: Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0)),
        ..style!("m-2")
    }) />
</MiniNodeBundle>
<MiniNodeBundle Style=(Style{
    margin:UiRect::left(Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0))),
    ..style!("absolute w-0 h-full flex-col align-items:center justify-content:center align-self:center")
}) >
    <MiniNodeBundle @id="handle" @style="absolute align-self:center w/h-1.0 h-80% min-w-16 min-h-16"
        ThemeComponent=(ThemeComponent::new(StyleFlags::default(), WidgetKind::SliderHandle)) />
</MiniNodeBundle>
}
