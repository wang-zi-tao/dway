use bevy::{ecs::system::SystemId, ui::FocusPolicy};

use crate::prelude::*;

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub enum UiButtonEventKind {
    Pressed,
    Released,
    Hovered,
    Leaved,
}

#[derive(Debug, Clone)]
pub struct UiButtonEvent {
    pub kind: UiButtonEventKind,
    pub receiver: Entity,
    pub button: Entity,
}

#[derive(Component, Default, Clone, Copy, Reflect)]
pub struct UiButton {
    #[reflect(ignore)]
    pub callback: Option<(Entity, SystemId<UiButtonEvent>)>,
    pub state: Interaction,
}

impl UiButton {
    pub fn new(receiver: Entity, callback: SystemId<UiButtonEvent>) -> Self {
        Self {
            callback: Some((receiver, callback)),
            state: Interaction::None,
        }
    }
}

pub fn process_ui_button_event(
    mut ui_query: Query<(Entity, &mut UiButton, &Interaction), Changed<Interaction>>,
    mut commands: Commands,
) {
    use UiButtonEventKind::*;
    ui_query.for_each_mut(|(entity, mut button, button_state)| {
        let mut call = |kind| {
            if let Some((receiver, callback)) = &button.callback {
                commands.run_system_with_input(
                    *callback,
                    UiButtonEvent {
                        kind,
                        receiver:*receiver,
                        button: entity,
                    },
                );
            }
        };
        match (button.state, button_state) {
            (Interaction::Pressed, Interaction::Hovered) => {
                call(Released);
            }
            (Interaction::Pressed, Interaction::None) => {
                call(Released);
                call(Leaved);
            }
            (Interaction::Hovered, Interaction::Pressed) => {
                call(Pressed);
            }
            (Interaction::Hovered, Interaction::None) => {
                call(Leaved);
            }
            (Interaction::None, Interaction::Pressed) => {
                call(Hovered);
                call(Pressed);
            }
            (Interaction::None, Interaction::Hovered) => {
                call(Hovered);
            }
            (Interaction::None, Interaction::None)
            | (Interaction::Hovered, Interaction::Hovered)
            | (Interaction::Pressed, Interaction::Pressed) => {}
        };
        button.state = *button_state;
    });
}

#[derive(Bundle, Default)]
pub struct UiButtonAddonBundle {
    pub button: UiButton,
    pub interaction: Interaction,
}

impl From<UiButton> for UiButtonAddonBundle {
    fn from(value: UiButton) -> Self {
        Self {
            button: value,
            interaction: Default::default(),
        }
    }
}

impl UiButtonAddonBundle {
    pub fn new(receiver: Entity, callback: SystemId<UiButtonEvent>) -> Self {
        Self {
            button: UiButton::new(receiver, callback),
            interaction: Default::default(),
        }
    }
}

#[derive(Bundle, Default)]
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
