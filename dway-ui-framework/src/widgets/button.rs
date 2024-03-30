use bevy_relationship::reexport::SmallVec;
// use bevy_tweening::{AssetAnimator, EaseMethod};
use crate::{
    make_bundle,
    prelude::*,
    theme::{StyleFlags, ThemeComponent, WidgetKind},
};
use smart_default::SmartDefault;

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
    pub fn with_callback(receiver: Entity, system: SystemId<UiButtonEvent>) -> Self {
        Self {
            callback: SmallVec::from_slice(&[(receiver, system)]),
            state: Interaction::None,
        }
    }
    pub fn with_callbacks(callbacks: &[(Entity, SystemId<UiButtonEvent>)]) -> Self {
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
    mut ui_query: Query<
        (
            Entity,
            &mut UiButton,
            &Interaction,
            Option<&mut ThemeComponent>,
        ),
        Changed<Interaction>,
    >,
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
            theme
                .style_flags
                .set(StyleFlags::HOVERED, button.state == Interaction::Hovered);
            theme
                .style_flags
                .set(StyleFlags::CLICKED, button.state == Interaction::Pressed);
        }
    }
}

make_bundle! {
    @from button: UiButton,
    @addon UiRawButtonExt,
    UiRawButtonBundle {
        pub button: UiButton,
        pub interaction: Interaction,
        #[default(ThemeComponent::none())]
        pub theme: ThemeComponent,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
    }
}
make_bundle! {
    @from button: UiButton,
    @addon UiButtonExt,
    UiButtonBundle {
        pub button: UiButton,
        pub interaction: Interaction,
        #[default(ThemeComponent::new(StyleFlags::default(), WidgetKind::Button))]
        pub theme: ThemeComponent,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
    }
}
make_bundle! {
    @from button: UiButton,
    @addon UiHightlightButtonExt,
    UiHightlightButtonBundle {
        pub button: UiButton,
        pub interaction: Interaction,
        #[default(ThemeComponent::new(StyleFlags::HIGHLIGHT, WidgetKind::Button))]
        pub theme: ThemeComponent,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
    }
}

impl UiButtonExt {
    pub fn new(receiver: Entity, callback: SystemId<UiButtonEvent>) -> Self {
        Self {
            button: UiButton::new(receiver, callback),
            ..default()
        }
    }
    pub fn from_slice(callbacks: &[(Entity, SystemId<UiButtonEvent>)]) -> Self {
        Self {
            button: UiButton::with_callbacks(callbacks),
            ..default()
        }
    }
}
