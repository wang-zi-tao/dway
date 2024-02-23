use bevy::{input::keyboard::KeyboardInput, ui::RelativeCursorPosition};
use bevy_relationship::reexport::SmallVec;

use crate::{prelude::*, theme::ThemeComponent};

pub type Callback<E> = (Entity, SystemId<E>);
pub type CallbackSlot<E> = Option<(Entity, SystemId<E>)>;
pub type CallbackSlots<E> = SmallVec<[(Entity, SystemId<E>); 2]>;

#[derive(Resource, Default, Reflect, Debug)]
pub struct MousePosition {
    pub window: Option<Entity>,
    pub position: Option<Vec2>,
}

pub fn update_mouse_position(
    mut mouse_event: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(mouse) = mouse_event.read().last() {
        mouse_position.window = Some(mouse.window);
        mouse_position.position = Some(mouse.position);
    }
}

structstruck::strike! {
    pub struct UiInputEvent{
        pub receiver: Entity,
        pub node: Entity,
        pub event:
        #[derive(Clone)]
        pub enum UiInputEventKind{
            MouseEnter,
            MouseLeave,
            MousePress,
            MouseRelease,
            KeybordEnter,
            KeyboardLeave,
            MouseMove(Vec2),
            KeyboardInput(KeyboardInput),
        }
    }
}

#[derive(Component, Debug)]
pub struct UiInput {
    pub callbacks: SmallVec<[(Entity, SystemId<UiInputEvent>); 2]>,
    pub mouse_focused: bool,
    pub input_focused: bool,
    pub input_grabed: bool,
    pub self_interaction: Interaction,
    pub mouse_state: Interaction,
}

#[derive(Resource)]
pub struct UiFocusState {
    pub mouse_focus: Option<Entity>,
    pub input_focus: Option<Entity>,
}

pub fn update_ui_input(
    mut query: Query<
        (
            Entity,
            &UiInput,
            Ref<Interaction>,
            Option<&mut ThemeComponent>,
            Option<Ref<RelativeCursorPosition>>,
        ),
        Or<(
            Changed<UiInput>,
            Changed<Interaction>,
            Changed<RelativeCursorPosition>,
        )>,
    >,
    mut commands: Commands,
    mut keyboard_event: EventReader<KeyboardInput>,
) {
    let mut run_callback = |kind: UiInputEventKind, entity: Entity, ui_focus: &UiInput| {
        for (receiver, callback) in &ui_focus.callbacks {
            commands.run_system_with_input(
                *callback,
                UiInputEvent {
                    receiver: *receiver,
                    node: entity,
                    event: kind.clone(),
                },
            );
        }
    };
    for (entity, ui_focus, interaction, theme_component, relative_cursor_position) in &mut query {
        use UiInputEventKind::*;
        let mut call = |kind: UiInputEventKind| {
            run_callback(kind, entity, ui_focus);
        };
        if let Some(relative_cursor_position) = relative_cursor_position.as_ref() {
            if relative_cursor_position.is_changed() {
                if let Some(pos) = relative_cursor_position.normalized {
                    call(MouseMove(pos));
                }
            }
        }
        match (ui_focus.mouse_state, &*interaction) {
            (Interaction::Pressed, Interaction::Hovered) => {
                call(MouseRelease);
            }
            (Interaction::Pressed, Interaction::None) => {
                call(MouseRelease);
                call(MouseLeave);
            }
            (Interaction::Hovered, Interaction::Pressed) => {
                call(MousePress);
            }
            (Interaction::Hovered, Interaction::None) => {
                call(MouseLeave);
            }
            (Interaction::None, Interaction::Pressed) => {
                call(MouseEnter);
                call(MousePress);
            }
            (Interaction::None, Interaction::Hovered) => {
                call(MousePress);
            }
            (Interaction::None, Interaction::None)
            | (Interaction::Hovered, Interaction::Hovered)
            | (Interaction::Pressed, Interaction::Pressed) => {}
        };
    }
    for key in keyboard_event.read() {
        for (entity, ui_focus, ..) in &mut query {
            if ui_focus.input_focused || ui_focus.input_grabed {
                run_callback(
                    UiInputEventKind::KeyboardInput(key.clone()),
                    entity,
                    ui_focus,
                );
            }
        }
    }
}
