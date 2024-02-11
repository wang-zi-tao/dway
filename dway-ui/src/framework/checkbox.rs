use bevy_relationship::reexport::SmallVec;
// use bevy_tweening::AssetAnimator;
use smart_default::SmartDefault;

use crate::prelude::*;

use super::button::ButtonColor;

#[derive(Component, Default, Reflect)]
pub struct UiCheckBox {
    #[reflect(ignore)]
    pub callback: SmallVec<[(Entity, SystemId<UiCheckBoxEvent>); 2]>,
    pub state: Interaction,
    pub prev_state: Interaction,
}

#[derive(Component, Default, Reflect)]
pub struct UiCheckBoxState{
    pub value: bool,
}

impl UiCheckBoxState {
    pub fn new(value: bool) -> Self { Self { value } }
}

impl UiCheckBox {
    pub fn new(callback: Vec<(Entity, SystemId<UiCheckBoxEvent>)>) -> Self {
        Self {
            callback: callback.into(),
            state: default(),
            prev_state: default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiCheckBoxEventKind {
    Down,
    Up,
    Pressed,
    Released,
    Hovered,
    Leaved,
}

pub struct UiCheckBoxEvent {
    pub receiver: Entity,
    pub checkbox: Entity,
    pub kind: UiCheckBoxEventKind,
    pub value: bool,
    pub state: Interaction,
    pub prev_state: Interaction,
}

#[derive(Bundle, SmartDefault)]
pub struct UiCheckBoxAddonBundleWithoutState {
    pub checkbox: UiCheckBox,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

impl From<UiCheckBox> for UiCheckBoxAddonBundleWithoutState {
    fn from(value: UiCheckBox) -> Self {
        Self {
            checkbox: value,
            ..default()
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct UiCheckBoxAddonBundle {
    pub checkbox: UiCheckBox,
    pub state: UiCheckBoxState,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

impl From<UiCheckBox> for UiCheckBoxAddonBundle {
    fn from(value: UiCheckBox) -> Self {
        Self {
            checkbox: value,
            ..default()
        }
    }
}

pub fn process_ui_checkbox_event(
    mut ui_query: Query<(Entity, &mut UiCheckBox, &mut UiCheckBoxState, &Interaction), Changed<Interaction>>,
    mut commands: Commands,
) {
    ui_query.for_each_mut(|(entity, mut checkbox, mut state, button_state)| {
        use UiCheckBoxEventKind::*;
        let mut call = |state: &UiCheckBoxState, kind: UiCheckBoxEventKind| {
            for (receiver, callback) in &checkbox.callback {
                commands.run_system_with_input(
                    *callback,
                    UiCheckBoxEvent {
                        kind: kind.clone(),
                        receiver: *receiver,
                        checkbox: entity,
                        value: state.value,
                        state: *button_state,
                        prev_state: checkbox.state,
                    },
                );
            }
        };
        match (checkbox.state, button_state) {
            (Interaction::Pressed, Interaction::Hovered) => {
                call(&state, Released);
                state.value = !state.value;
                if state.value {
                    call(&state, UiCheckBoxEventKind::Down);
                } else {
                    call(&state, UiCheckBoxEventKind::Up);
                }
            }
            (Interaction::Pressed, Interaction::None) => {
                call(&state, Released);
                call(&state, Leaved);
                state.value = !state.value;
                if state.value {
                    call(&state, UiCheckBoxEventKind::Down);
                } else {
                    call(&state, UiCheckBoxEventKind::Up);
                }
            }
            (Interaction::Hovered, Interaction::Pressed) => {
                call(&state, Pressed);
            }
            (Interaction::Hovered, Interaction::None) => {
                call(&state, Leaved);
            }
            (Interaction::None, Interaction::Pressed) => {
                call(&state, Hovered);
                call(&state, Pressed);
            }
            (Interaction::None, Interaction::Hovered) => {
                call(&state, Hovered);
            }
            (Interaction::None, Interaction::None)
            | (Interaction::Hovered, Interaction::Hovered)
            | (Interaction::Pressed, Interaction::Pressed) => {}
        };
        checkbox.state = *button_state;
    });
}

// pub fn checkbox_color_callback<T>(
//     In(event): In<UiCheckBoxEvent>,
//     style_query: Query<&ButtonColor>,
//     mut commands: Commands,
// ) where
//     T: Asset,
//     ColorMaterialColorLens: Lens<T>,
// {
//     let Ok(style) = style_query.get(event.checkbox) else {
//         return;
//     };
//     let get_style = |state: &Interaction| match state {
//         Interaction::Pressed => &style.clicked,
//         Interaction::Hovered => &style.hover,
//         Interaction::None => {
//             if event.value {
//                 &style.clicked
//             } else {
//                 &style.normal
//             }
//         }
//     };
//     let tween = Tween::<T>::new(
//         style.animation_method,
//         style.animation_duration,
//         ColorMaterialColorLens {
//             start: get_style(&event.prev_state).clone(),
//             end: get_style(&event.state).clone(),
//         },
//     );
//     commands
//         .entity(event.checkbox)
//         // .insert(AssetAnimator::new(tween))
//         ;
// }

#[derive(Bundle, SmartDefault)]
pub struct RoundedCheckBoxAddonBundle {
    pub checkbox: UiCheckBox,
    pub state: UiCheckBoxState,
    pub interaction: Interaction,
    pub color: ButtonColor,
    pub material: Handle<RoundedUiRectMaterial>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

impl RoundedCheckBoxAddonBundle {
    pub fn new(
        mut checkbox: UiCheckBox,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        theme: &Theme,
        class: &str,
        entity: Entity,
    ) -> Self {
        // checkbox.callback.push((
        //     entity,
        //     theme.system(checkbox_color_callback::<RoundedUiRectMaterial>),
        // ));
        Self {
            checkbox,
            color: ButtonColor::from_theme(theme, class),
            interaction: Default::default(),
            material: rect_material_set.add(RoundedUiRectMaterial::new(theme.color("panel"), 8.0)),
            focus_policy: FocusPolicy::Block,
            state: UiCheckBoxState::default(),
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct RoundedCheckBoxAddonBundleWithoutState {
    pub checkbox: UiCheckBox,
    pub interaction: Interaction,
    pub color: ButtonColor,
    pub material: Handle<RoundedUiRectMaterial>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

impl RoundedCheckBoxAddonBundleWithoutState {
    pub fn new(
        mut checkbox: UiCheckBox,
        rect_material_set: &mut Assets<RoundedUiRectMaterial>,
        theme: &Theme,
        class: &str,
        entity: Entity,
    ) -> Self {
        // checkbox.callback.push((
        //     entity,
        //     theme.system(checkbox_color_callback::<RoundedUiRectMaterial>),
        // ));
        Self {
            checkbox,
            color: ButtonColor::from_theme(theme, class),
            interaction: Default::default(),
            material: rect_material_set.add(RoundedUiRectMaterial::new(theme.color("panel"), 8.0)),
            focus_policy: FocusPolicy::Block,
        }
    }
}
