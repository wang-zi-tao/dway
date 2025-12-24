use crate::{
    event::EventDispatcher,
    prelude::*,
    theme::{StyleFlags, ThemeComponent, WidgetKind},
};

#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub enum UiButtonEventKind {
    Pressed,
    Released,
    Hovered,
    Leaved,
}

pub type UiButtonEventDispatcher = EventDispatcher<UiButtonEvent>;

#[derive(Debug, Clone)]
pub struct UiButtonEvent {
    pub kind: UiButtonEventKind,
    pub state: Interaction,
    pub prev_state: Interaction,
}

#[derive(Component, Default, Clone, Reflect)]
#[require(Node, Interaction)]
#[require(FocusPolicy=FocusPolicy::Block)]
#[require(ThemeComponent=ThemeComponent::widget(WidgetKind::Button))]
pub struct UiButton {
    pub state: Interaction,
}

pub fn update_ui_button(
    mut ui_query: Query<
        (
            Entity,
            &mut UiButton,
            &Interaction,
            &UiButtonEventDispatcher,
            Option<&mut ThemeComponent>,
        ),
        Changed<Interaction>,
    >,
    mut commands: Commands,
) {
    use UiButtonEventKind::*;
    for (entity, mut button, button_state, dispatcher, theme) in &mut ui_query {
        let mut call = |kind: UiButtonEventKind| {
            dispatcher.send(
                UiButtonEvent {
                    kind: kind.clone(),
                    state: *button_state,
                    prev_state: button.state,
                },
                &mut commands,
            );
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
