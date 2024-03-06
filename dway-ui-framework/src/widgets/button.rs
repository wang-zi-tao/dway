use bevy::{ecs::system::SystemId, ui::FocusPolicy};
use bevy_relationship::reexport::SmallVec;
// use bevy_tweening::{AssetAnimator, EaseMethod};
use smart_default::SmartDefault;

use crate::{prelude::*, theme::{StyleFlags, ThemeComponent, WidgetKind}};

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
    pub state: Interaction,
    pub prev_state: Interaction,
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
    pub fn from_slice(callbacks: &[(Entity, SystemId<UiButtonEvent>)]) -> Self {
        Self {
            callback: SmallVec::from_slice(callbacks),
            state: Interaction::None,
        }
    }
    pub fn register_callback(&mut self, callback: Callback<UiButtonEvent>) {
        self.callback.push(callback);
    }
}

pub fn process_ui_button_event(
    mut ui_query: Query<(Entity, &mut UiButton, &Interaction, Option<&mut ThemeComponent>), Changed<Interaction>>,
    mut commands: Commands,
) {
    use UiButtonEventKind::*;
    for (entity, mut button, button_state, theme) in &mut ui_query {
        let mut call = |kind: UiButtonEventKind| {
            for (receiver, callback) in &button.callback {
                commands.run_system_with_input(
                    *callback,
                    UiButtonEvent {
                        kind: kind.clone(),
                        receiver: *receiver,
                        button: entity,
                        state: *button_state,
                        prev_state: button.state,
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

        if let Some(mut theme) = theme {
            theme.style_flags.set(StyleFlags::HOVERED, button.state == Interaction::Hovered);
            theme.style_flags.set(StyleFlags::CLICKED, button.state == Interaction::Pressed);
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct UiButtonAddonBundle {
    pub button: UiButton,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
    #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Button))]
    pub theme: ThemeComponent,
}

impl From<UiButton> for UiButtonAddonBundle {
    fn from(value: UiButton) -> Self {
        Self {
            button: value,
            ..default()
        }
    }
}

impl UiButtonAddonBundle {
    pub fn new(receiver: Entity, callback: SystemId<UiButtonEvent>) -> Self {
        Self {
            button: UiButton::new(receiver, callback),
            ..default()
        }
    }
    pub fn from_slice(callbacks: &[(Entity, SystemId<UiButtonEvent>)]) -> Self {
        Self {
            button: UiButton::from_slice(callbacks),
            ..default()
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct UiButtonBundle {
    pub button: UiButton,
    pub interaction: Interaction,
    #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Button))]
    pub theme: ThemeComponent,

    pub node: Node,
    pub style: Style,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

impl From<UiButton> for UiButtonBundle {
    fn from(button: UiButton) -> Self {
        Self {
            button,
            ..default()
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct ButtonAddonBundle<M: UiMaterial> {
    pub button: UiButton,
    pub interaction: Interaction,
    pub material: Handle<M>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}
