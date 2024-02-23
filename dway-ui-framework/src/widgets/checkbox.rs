use super::button::ButtonColor;
use crate::{make_bundle, prelude::*, theme::{StyleFlags, ThemeComponent, WidgetKind}};
use bevy_relationship::reexport::SmallVec;
use smart_default::SmartDefault;

#[derive(Component, Default, Reflect)]
pub struct UiCheckBox {
    #[reflect(ignore)]
    pub callback: SmallVec<[(Entity, SystemId<UiCheckBoxEvent>); 2]>,
    pub state: Interaction,
    pub prev_state: Interaction,
}

#[derive(Component, Default, Reflect)]
pub struct UiCheckBoxState {
    pub value: bool,
}

impl UiCheckBoxState {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl UiCheckBox {
    pub fn register_callback(&mut self, callback: Callback<UiCheckBoxEvent>) {
        self.callback.push(callback);
    }
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
    mut ui_query: Query<
        (Entity, &mut UiCheckBox, &mut UiCheckBoxState, &Interaction, Option<&mut ThemeComponent>),
        Changed<Interaction>,
    >,
    mut commands: Commands,
) {
    for (entity, mut checkbox, mut state, button_state, theme) in ui_query.iter_mut() {
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

        if let Some(mut theme) = theme {
            theme.style_flags.set(StyleFlags::HOVERED, checkbox.state == Interaction::Hovered);
            theme.style_flags.set(StyleFlags::CLICKED, checkbox.state == Interaction::Pressed);
            theme.style_flags.set(StyleFlags::DOWNED, state.value);
        }
    }
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
pub struct CheckBoxAddonBundle<M: UiMaterial> {
    pub checkbox: UiCheckBox,
    pub state: UiCheckBoxState,
    pub interaction: Interaction,
    pub color: ButtonColor,
    pub material: Handle<M>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,

    #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Checkbox))]
    pub theme: ThemeComponent,
}

#[derive(Bundle, SmartDefault)]
pub struct RoundedCheckBoxAddonBundleWithoutState<M: UiMaterial> {
    pub checkbox: UiCheckBox,
    pub interaction: Interaction,
    pub color: ButtonColor,
    pub material: Handle<M>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

make_bundle! {
    UiCheckboxBundle{
        pub checkbox: UiCheckBox,
        pub state: UiCheckBoxState,
        pub interaction: Interaction,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,

        #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Checkbox))]
        pub theme: ThemeComponent,
    }
}
