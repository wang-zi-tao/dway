use bevy::{ecs::system::SystemId, ui::FocusPolicy};

use crate::prelude::*;

#[derive(Event)]
pub enum UiButtonEvent {
    Pressed(Entity),
    Released(Entity),
    Hovered(Entity),
    Leaved(Entity),
}

#[derive(Component, Default, Clone, Copy, Reflect)]
pub struct UiButton {
    #[reflect(ignore)]
    pub callback: Option<SystemId>,
    pub state: Interaction,
}

pub fn process_ui_button_event(
    mut ui_query: Query<(Entity, &mut UiButton, &Interaction), Changed<Interaction>>,
    mut events: EventWriter<UiButtonEvent>,
    mut commands: Commands,
) {
    ui_query.for_each_mut(|(entity, mut button, button_state)| {
        match (button.state, button_state) {
            (Interaction::Pressed, Interaction::Hovered) => {
                events.send(UiButtonEvent::Released(entity));
            }
            (Interaction::Pressed, Interaction::None) => {
                events.send(UiButtonEvent::Released(entity));
                events.send(UiButtonEvent::Leaved(entity));
            }
            (Interaction::Hovered, Interaction::Pressed) => {
                events.send(UiButtonEvent::Pressed(entity));
            }
            (Interaction::Hovered, Interaction::None) => {
                events.send(UiButtonEvent::Leaved(entity));
            }
            (Interaction::None, Interaction::Pressed) => {
                events.send(UiButtonEvent::Hovered(entity));
                events.send(UiButtonEvent::Pressed(entity));
            }
            (Interaction::None, Interaction::Hovered) => {
                events.send(UiButtonEvent::Hovered(entity));
            }
            (Interaction::None, Interaction::None)
            | (Interaction::Hovered, Interaction::Hovered)
            | (Interaction::Pressed, Interaction::Pressed) => {}
        };
        if button.state == *button_state {
            if let Some(callback) = button.callback {
                commands.run_system(callback);
            }
        }
        button.state = *button_state;
    });
}

#[derive(Bundle)]
pub struct UiButtonBundle {
    pub button: UiButton,
    pub interaction: Interaction,

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
