use bevy::{ecs::system::SystemId, ui::FocusPolicy};
use bevy_relationship::reexport::SmallVec;
use bevy_tweening::{AssetAnimator, EaseMethod};
use smart_default::SmartDefault;

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
    });
}

#[derive(Bundle, SmartDefault)]
pub struct UiButtonAddonBundle {
    pub button: UiButton,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
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

#[derive(Component, Reflect, Default)]
pub struct ButtonColor {
    pub normal: Color,
    pub hover: Color,
    pub clicked: Color,
    #[reflect(ignore)]
    pub animation_method: EaseMethod,
    pub animation_duration: Duration,
}

impl ButtonColor {
    pub fn new(normal: Color, hover: Color, clicked: Color) -> Self {
        Self {
            normal,
            hover,
            clicked,
            animation_method: EaseMethod::Linear,
            animation_duration: Duration::from_secs_f32(0.15),
        }
    }
    pub fn from_theme(theme: &Theme, class: &str) -> Self {
        Self::new(
            theme.color(&format!("{class}")),
            theme.color(&format!("{class}:hover")),
            theme.color(&format!("{class}:clicked")),
        )
    }
}

impl ButtonColor {
    pub fn callback_system<T>(
        In(event): In<UiButtonEvent>,
        style_query: Query<&Self>,
        mut commands: Commands,
    ) where
        T: Asset,
        ColorMaterialColorLens: Lens<T>,
    {
        let Ok(style) = style_query.get(event.button) else {
            return;
        };
        let get_style = |state: &Interaction| match state {
            Interaction::Pressed => &style.clicked,
            Interaction::Hovered => &style.hover,
            Interaction::None => &style.normal,
        };
        let tween = Tween::<T>::new(
            style.animation_method,
            style.animation_duration,
            ColorMaterialColorLens {
                start: get_style(&event.prev_state).clone(),
                end: get_style(&event.state).clone(),
            },
        );
        commands
            .entity(event.button)
            .insert(AssetAnimator::new(tween));
    }
}

#[derive(Bundle, SmartDefault)]
pub struct RoundedButtonAddonBundle {
    pub button: UiButton,
    pub interaction: Interaction,
    pub color: ButtonColor,
    pub material: Handle<RoundedUiRectMaterial>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}
