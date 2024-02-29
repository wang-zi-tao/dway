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

#[derive(Component, Debug, Default)]
pub struct UiInput {
    pub callbacks: SmallVec<[(Entity, SystemId<UiInputEvent>); 2]>,
    pub mouse_focused: bool,
    pub input_focused: bool,
    pub input_grabed: bool,
    pub self_interaction: Interaction,
    pub mouse_state: Interaction,
}

impl UiInput {
    pub fn can_receive_keyboard_input(&self) -> bool {
        self.input_focused || self.input_grabed
    }
}

#[derive(Event, Reflect)]
pub enum UiFocusEvent {
    FocusLeaveRequest(Entity),
    FocusEnterRequest(Entity),
}

#[derive(Resource, Reflect, Default)]
pub struct UiFocusState {
    pub mouse_focus: Option<Entity>,
    pub input_focus: Option<Entity>,
}

pub fn update_ui_input(
    mut query: Query<(
        Entity,
        &mut UiInput,
        Ref<Interaction>,
        Option<&mut ThemeComponent>,
        Option<Ref<RelativeCursorPosition>>,
    )>,
    mut commands: Commands,
    mut keyboard_event: EventReader<KeyboardInput>,
    mut ui_focus_event: EventReader<UiFocusEvent>,
    mut ui_focus_state: ResMut<UiFocusState>,
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
        if !interaction.is_changed()
            && !ui_focus.is_changed()
            && relative_cursor_position
                .as_ref()
                .map(|r| r.is_changed())
                .unwrap_or_default()
        {
            continue;
        }

        use UiInputEventKind::*;
        let mut call = |kind: UiInputEventKind| {
            run_callback(kind, entity, &ui_focus);
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
                    &ui_focus,
                );
            }
        }
    }

    for event in ui_focus_event.read() {
        match event {
            UiFocusEvent::FocusLeaveRequest(e) => {
                if let Ok((_, mut ui_focus, ..)) = query.get_mut(*e) {
                    ui_focus.input_focused = false;
                    run_callback(UiInputEventKind::KeyboardLeave, *e, &ui_focus);
                }
                ui_focus_state.input_focus = None;
            }
            UiFocusEvent::FocusEnterRequest(e) => {
                if let Some((_, mut ui_focus, ..)) = ui_focus_state
                    .input_focus
                    .and_then(|old_node| query.get_mut(old_node).ok())
                {
                    ui_focus.input_focused = false;
                    run_callback(UiInputEventKind::KeyboardLeave, *e, &ui_focus);
                }
                if let Ok((_, mut ui_focus, ..)) = query.get_mut(*e) {
                    ui_focus.input_focused = true;
                    run_callback(UiInputEventKind::KeybordEnter, *e, &ui_focus);
                }
                ui_focus_state.input_focus = Some(*e);
            }
        }
    }
}
