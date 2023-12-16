use smart_default::SmartDefault;
use super::MousePosition;
use crate::prelude::*;

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
@global(mouse_position:MousePosition)
@state_reflect()
@use_state(pub value: f32)
@bundle({
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy = FocusPolicy::Block,
    pub style: Style = style!("items-center absolute full min-h-16 min-w-32"),
})
@world_query(slider_node: &Node)
@world_query(slider_transform: &GlobalTransform)
@world_query(slider_interaction: Ref<Interaction>)
@before{
if ( slider_interaction.is_changed() || mouse_position.is_changed() )
        && *slider_interaction == Interaction::Pressed{
    let position = mouse_position.position.unwrap_or_default();
    let slider_rect = Rect::from_center_size(slider_transform.translation().xy(), slider_node.size());
    let raw_value = ((position.x - slider_rect.min.x)/slider_rect.size().x).max(0.0).min(1.0);
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
} }
<MiniNodeBundle @id="bar" @style="absolute h-8 w-full min-h-8 align-self:center"
    @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("slider:bar"), 4.0))
>
    <MiniNodeBundle @id="bar_highlight"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("slider:bar:highlight"), 4.0)) Style=(Style{
        width: Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0)),
        ..style!("m-2")
    }) />
</MiniNodeBundle>
<MiniNodeBundle Style=(Style{
    margin:UiRect::left(Val::Percent(100.0*((*state.value()-prop.min)/(prop.max-prop.min)).max(0.0).min(1.0))),
    ..style!("absolute w-0 h-full flex-col align-items:center justify-content:center align-self:center")
}) >
    <MiniNodeBundle @id="handle" @style="absolute align-self:center w/h-1.0 h-80% min-w-16 min-h-16"
        @material(UiCircleMaterial=>UiCircleMaterial::new(theme.color("slider:handle"), 8.0)) />
</MiniNodeBundle>
}
