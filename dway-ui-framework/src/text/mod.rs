use bevy::input::ButtonState;
use cursor::{
    text_cursor_on_input_system, update_text_cursor_layout_system, UiTextCursor, UiTextCursorEvent,
};
use editor::UiTextEditor;
use selection::{update_ui_text_selection_system, UiTextSelection};
use textarea::{update_textarea, UiTextArea};

use crate::{event::EventReceiver, prelude::*};

pub mod cursor;
pub mod editor;
pub mod selection;
pub mod textarea;

#[derive(Clone)]
pub enum UiTextEvent {
    ChangePosition {
        position: Vec2,
        byte_index: usize,
    },
    TextLayoutChanged {},
    Click {
        button: MouseButton,
        state: ButtonState,
        entity: Entity,
        byte_index: usize,
    },
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UiTextSystem {
    UpdateTextArea,
    UpdateCursor,
    UpdateSelectionArea,
}

#[derive(Default)]
pub struct UiTextPlugin;

impl Plugin for UiTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                update_textarea.in_set(UiTextSystem::UpdateTextArea),
                update_text_cursor_layout_system
                    .in_set(UiTextSystem::UpdateCursor)
                    .after(UiTextSystem::UpdateTextArea),
                update_ui_text_selection_system
                    .in_set(UiTextSystem::UpdateSelectionArea)
                    .after(UiTextSystem::UpdateCursor),
            ),
        )
        .register_type::<UiTextArea>()
        .register_type::<UiTextCursor>()
        .register_type::<UiTextSelection>()
        .register_component_as::<dyn EventReceiver<UiInputEvent>, UiTextCursor>()
        .register_component_as::<dyn EventReceiver<UiTextCursorEvent>, UiTextSelection>()
        .register_component_as::<dyn EventReceiver<UiInputEvent>, UiTextEditor>()
        .register_callback(text_cursor_on_input_system);
    }
}
