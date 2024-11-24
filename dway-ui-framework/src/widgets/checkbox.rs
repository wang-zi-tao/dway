use bevy_relationship::reexport::SmallVec;
use smart_default::SmartDefault;

use crate::{
    make_bundle,
    prelude::*,
    theme::{StyleFlags, ThemeComponent, WidgetKind},
};

#[derive(Component, Default, Reflect)]
pub struct UiCheckBox {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UiCheckBoxEventKind {
    Down,
    Up,
    Pressed,
    Released,
    Hovered,
    Leaved,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UiCheckBoxEvent {
    pub kind: UiCheckBoxEventKind,
    pub value: bool,
}

pub type UiCheckBoxEventDispatcher = EventDispatcher<UiCheckBoxEvent>;

#[derive(Bundle, SmartDefault)]
pub struct UiCheckBoxExtWithoutState {
    pub checkbox: UiCheckBox,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}

impl From<UiCheckBox> for UiCheckBoxExtWithoutState {
    fn from(value: UiCheckBox) -> Self {
        Self {
            checkbox: value,
            ..default()
        }
    }
}

make_bundle! {
    @from checkbox: UiCheckBox,
    @addon UiCheckBoxExt,
    UiCheckBoxBundle {
        pub checkbox: UiCheckBox,
        pub state: UiCheckBoxState,
        pub interaction: Interaction,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
        #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Checkbox))]
        pub theme: ThemeComponent,
        pub event_dispatcher: UiCheckBoxEventDispatcher,
    }
}

pub fn update_ui_checkbox(
    mut ui_query: Query<
        (
            Entity,
            &mut UiCheckBox,
            &mut UiCheckBoxState,
            &Interaction,
            &UiCheckBoxEventDispatcher,
            Option<&mut ThemeComponent>,
        ),
        Changed<Interaction>,
    >,
    mut commands: Commands,
) {
    for (entity, mut checkbox, mut state, button_state, event_dispatcher, theme) in
        ui_query.iter_mut()
    {
        use UiCheckBoxEventKind::*;
        match (checkbox.state, button_state) {
            (Interaction::Pressed, Interaction::Hovered) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Released,
                        value: state.value,
                    },
                    &mut commands,
                );
                state.value = !state.value;
                if state.value {
                    event_dispatcher.send(
                        UiCheckBoxEvent {
                            kind: Down,
                            value: state.value,
                        },
                        &mut commands,
                    );
                } else {
                    event_dispatcher.send(
                        UiCheckBoxEvent {
                            kind: Up,
                            value: state.value,
                        },
                        &mut commands,
                    );
                }
            }
            (Interaction::Pressed, Interaction::None) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Released,
                        value: state.value,
                    },
                    &mut commands,
                );
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Leaved,
                        value: state.value,
                    },
                    &mut commands,
                );
                state.value = !state.value;
                if state.value {
                    event_dispatcher.send(
                        UiCheckBoxEvent {
                            kind: Down,
                            value: state.value,
                        },
                        &mut commands,
                    );
                } else {
                    event_dispatcher.send(
                        UiCheckBoxEvent {
                            kind: Up,
                            value: state.value,
                        },
                        &mut commands,
                    );
                }
            }
            (Interaction::Hovered, Interaction::Pressed) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Pressed,
                        value: state.value,
                    },
                    &mut commands,
                );
            }
            (Interaction::Hovered, Interaction::None) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Leaved,
                        value: state.value,
                    },
                    &mut commands,
                );
            }
            (Interaction::None, Interaction::Pressed) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Hovered,
                        value: state.value,
                    },
                    &mut commands,
                );
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Pressed,
                        value: state.value,
                    },
                    &mut commands,
                );
            }
            (Interaction::None, Interaction::Hovered) => {
                event_dispatcher.send(
                    UiCheckBoxEvent {
                        kind: Hovered,
                        value: state.value,
                    },
                    &mut commands,
                );
            }
            (Interaction::None, Interaction::None)
            | (Interaction::Hovered, Interaction::Hovered)
            | (Interaction::Pressed, Interaction::Pressed) => {}
        };
        checkbox.state = *button_state;

        if let Some(mut theme) = theme {
            theme
                .style_flags
                .set(StyleFlags::HOVERED, checkbox.state == Interaction::Hovered);
            theme
                .style_flags
                .set(StyleFlags::CLICKED, checkbox.state == Interaction::Pressed);
            theme.style_flags.set(StyleFlags::DOWNED, state.value);
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct CheckBoxAddonBundle<M: UiMaterial> {
    pub checkbox: UiCheckBox,
    pub state: UiCheckBoxState,
    pub interaction: Interaction,
    pub material: Handle<M>,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,

    #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Checkbox))]
    pub theme: ThemeComponent,
}
