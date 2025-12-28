use bevy::input::keyboard::Key;
use undo::History;

use super::{
    cursor::{UiTextCursor, UiTextCursorEvent},
    selection::UiTextSelection,
    textarea::UiTextArea,
};
use crate::{impl_event_receiver, prelude::*};

pub enum UiTextEditorEvent {
    Changed(String),
}

#[derive(Debug)]
pub enum UiInputCommand {
    Insert(usize, String),
    Delete(usize, String),
    Replace(usize, String, String),
}
impl UiInputCommand {
    pub fn new_delete_char(textarea: &UiTextArea, cursor: &UiTextCursor) -> Self {
        let char = textarea
            .data
            .char_indices()
            .find_map(|(pos, c)| (pos == cursor.byte_index).then(|| c.to_string()));
        Self::Delete(cursor.byte_index, char.unwrap_or_default())
    }

    pub fn apply(&self, textarea: &mut UiTextArea) {
        match self {
            UiInputCommand::Insert(p, d) => {
                textarea.data.insert_str(*p, d);
            }
            UiInputCommand::Delete(p, d) => {
                let split_off = textarea.data.split_off(*p);
                textarea.data.push_str(split_off.split_at(d.len()).1);
            }
            UiInputCommand::Replace(p, remove, insert) => {
                let split_off = textarea.data.split_off(*p);
                textarea.data.push_str(insert);
                textarea.data.push_str(split_off.split_at(remove.len()).1);
            }
        }
    }
}

#[derive(Component, SmartDefault)]
#[require(UiTextCursor)]
pub struct UiTextEditor {
    pub histtory: History<UiInputCommand>,
}

impl UiTextEditor {
    pub fn insert_text(
        &mut self,
        textarea: &mut UiTextArea,
        cursor: &mut UiTextCursor,
        text: &str,
    ) {
        UiInputCommand::Insert(cursor.byte_index, text.to_string()).apply(textarea);
        let position = textarea.data.floor_char_boundary(
            cursor.byte_index + text.len(),
        );
        cursor.byte_index = position;
    }

    pub fn backspace(&mut self, textarea: &mut UiTextArea, cursor: &mut UiTextCursor) {
        if cursor.byte_index > 0 {
            self.goto_previous_char(textarea, cursor);
            self.delete_char(textarea, cursor);
        }
    }

    pub fn delete_char(&mut self, textarea: &mut UiTextArea, cursor: &mut UiTextCursor) {
        UiInputCommand::new_delete_char(textarea, cursor).apply(textarea);
    }

    pub fn goto_previous_char(&mut self, textarea: &UiTextArea, cursor: &mut UiTextCursor) {
        cursor.byte_index = textarea
            .data
            .floor_char_boundary(cursor.byte_index.saturating_sub(1));
    }

    pub fn goto_next_char(&mut self, textarea: &UiTextArea, cursor: &mut UiTextCursor) {
        cursor.byte_index = textarea
            .data
            .floor_char_boundary(cursor.byte_index.saturating_add(1));
    }
}

pub fn text_editor_on_event(
    event: UiEvent<UiInputEvent>,
    mut query: Query<(
        Entity,
        &mut UiTextEditor,
        &mut UiTextSelection,
        &mut UiTextCursor,
        Option<&EventDispatcher<UiTextCursorEvent>>,
        &mut UiTextArea,
    )>,
    mut commands: Commands,
) {
    let Ok((_entity, mut editor, _selection, mut cursor, cursor_event, mut textarea)) =
        query.get_mut(event.sender())
    else {
        return;
    };

    if let UiInputEvent::KeyboardInput(keyboard_input) = &*event {
        if keyboard_input.state.is_pressed() {
            return;
        }

        match &keyboard_input.logical_key {
            Key::Backspace => {
                editor.backspace(&mut textarea, &mut cursor);
            }
            Key::Delete => {
                editor.delete_char(&mut textarea, &mut cursor);
            }
            Key::Enter => {
                editor.insert_text(&mut textarea, &mut cursor, "\n");
            }
            Key::Space => {
                editor.insert_text(&mut textarea, &mut cursor, " ");
            }
            Key::Character(s) => {
                editor.insert_text(&mut textarea, &mut cursor, s);
            }
            _ => {}
        }
    }
    if cursor.is_changed() {
        EventDispatcher::try_send(
            cursor_event,
            cursor.create_change_event(),
            event.sender(),
            &mut commands,
        );
    }
}

impl_event_receiver! {
    impl EventReceiver<UiInputEvent> for UiTextEditor => text_editor_on_event
}
