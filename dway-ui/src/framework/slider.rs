use smart_default::SmartDefault;

use crate::prelude::*;

use super::{
    button::{UiButtonBundle, UiButtonEvent, UiButtonEventKind},
    MiniButtonBundle,
};

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
@use_state(pub value: f32<=prop.min)
@arg(handle_query: Query<(&Node, &GlobalTransform, &Interaction)>)
@arg(mut mouse_event: EventReader<CursorMoved>)
@world_query(slider_transform: &GlobalTransform)
@world_query(slider_node: &Node)
@before{
    if let Ok((node,global,interaction)) = handle_query.get(node!(handle)) {
        if *interaction == Interaction::Pressed {
            let slider_rect = Rect::from_center_size(slider_transform.translation().xy(), slider_node.size());
            if let Some(mouse) = mouse_event.read().last() {
                let pos = mouse.position;
                let raw_value = ((mouse.position.x - slider_rect.min.x)/slider_rect.size().x).max(0.0).min(1.0);
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
        };
    };
}
<MiniButtonBundle @style="items-center absolute full min-h-16 min-w-32" >
    <MiniNodeBundle @id="bar" @style="absolute h-8 w-full min-h-8"
        @material(RoundedUiRectMaterial=>RoundedUiRectMaterial::new(theme.color("slider:bar"), 4.0))
    />
    <MiniNodeBundle Style=(Style{
        margin:UiRect::left(Val::Percent(100.0*(*state.value()-prop.min)/(prop.max-prop.min))),
        ..style!("absolute w-0 h-full flex-col align-items:center justify-content:center")
    }) >
        <MiniButtonBundle @id="handle" @style="absolute align-self:center w/h-1.0 h-80% min-w-16 min-h-16"
            @material(UiCircleMaterial=>UiCircleMaterial::new(theme.color("slider:handle"), 8.0)) />
    </MiniNodeBundle>
</MiniButtonBundle>
}
