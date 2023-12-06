use bevy::{ecs::system::SystemId, ui::FocusPolicy};
use bevy_relationship::reexport::SmallVec;

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

#[derive(Component, Default, Clone, Reflect)]
pub struct UiButton {
    #[reflect(ignore)]
    pub callback: SmallVec<[(Entity, SystemId<UiButtonEvent>); 2]>,
    pub state: Interaction,
}

impl UiButton {
    pub fn new(receiver: Entity, callback: SystemId<UiButtonEvent>) -> Self {
        Self {
            callback: SmallVec::from_slice(&[(receiver, callback)]),
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
        let mut call = |kind: UiButtonEventKind| {
            for (receiver, callback) in &button.callback {
                commands.run_system_with_input(
                    *callback,
                    UiButtonEvent {
                        kind: kind.clone(),
                        receiver: *receiver,
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

impl Default for UiButtonBundle {
    fn default() -> Self {
        Self {
            button: Default::default(),
            interaction: Default::default(),
            node: Default::default(),
            style: Default::default(),
            focus_policy: FocusPolicy::Block,
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}
