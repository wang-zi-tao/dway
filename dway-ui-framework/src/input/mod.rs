use bevy::{
    ecs::{event::EventCursor, query::QueryData},
    input::{keyboard::KeyboardInput, mouse::{MouseButtonInput, MouseWheel}},
    ui::RelativeCursorPosition,
};
use bevy_relationship::reexport::SmallVec;

use crate::{
    event::EventDispatcher,
    prelude::*,
    theme::{StyleFlags, ThemeComponent},
};

pub type Callback<E> = (Entity, SystemId<E>);
pub type CallbackSlot<E> = Option<(Entity, SystemId<E>)>;
pub type CallbackSlots<E> = SmallVec<[(Entity, SystemId<E>); 2]>;

#[derive(Resource, Default, Reflect, Debug)]
pub struct MousePosition {
    pub window: Option<Entity>,
    pub position: Option<Vec2>,
}

pub fn update_mouse_position(
    mut mouse_event: MessageReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(mouse) = mouse_event.read().last() {
        mouse_position.window = Some(mouse.window);
        mouse_position.position = Some(mouse.position);
    }
}

#[derive(Debug, Clone, Reflect)]
pub enum UiInputEvent {
    MouseEnter,
    MouseLeave,
    MousePress(MouseButton),
    MouseRelease(MouseButton),
    KeyboardEnter,
    KeyboardLeave,
    MouseMove(Vec2),
    KeyboardInput(KeyboardInput),
    Wheel(MouseWheel),
    RawMouseButton(MouseButtonInput),
}

impl UiInputEvent {
    pub fn interaction(&self) -> Option<Interaction> {
        match self {
            UiInputEvent::MouseEnter => Some(Interaction::Hovered),
            UiInputEvent::MouseLeave => Some(Interaction::None),
            UiInputEvent::MousePress(_) => Some(Interaction::Pressed),
            UiInputEvent::MouseRelease(_) => Some(Interaction::Hovered),
            UiInputEvent::KeyboardEnter => None,
            UiInputEvent::KeyboardLeave => None,
            UiInputEvent::MouseMove(_) => None,
            UiInputEvent::KeyboardInput(_) => None,
            UiInputEvent::Wheel(_) => None,
            UiInputEvent::RawMouseButton(_) => None,
        }
    }

    pub fn key_focus(&self) -> Option<bool> {
        match self {
            UiInputEvent::KeyboardEnter => Some(true),
            UiInputEvent::KeyboardLeave => Some(false),
            _ => None,
        }
    }
}

#[derive(Component, Debug, SmartDefault, Reflect)]
#[require(Node, Interaction, UiInputEventDispatcher)]
pub struct UiInput {
    pub input_focused: bool,
    pub input_grabed: bool,
    #[default(true)]
    pub grab_mouse_when_focused: bool,
    pub prev_mouse_state: Interaction,
    pub mouse_state: Interaction,
}

impl UiInput {
    pub fn can_receive_keyboard_input(&self) -> bool {
        self.input_focused || self.input_grabed
    }

    pub fn set_mouse_state(&mut self, intereaction: Interaction) {
        self.prev_mouse_state = self.mouse_state;
        self.mouse_state = intereaction;
    }

    pub fn just_pressed(&self) -> bool {
        self.mouse_state == Interaction::Pressed && self.prev_mouse_state != Interaction::Pressed
    }

    pub fn pressed(&self) -> bool {
        self.mouse_state == Interaction::Pressed
    }
}

#[derive(Message, Reflect)]
pub enum UiFocusEvent {
    FocusLeaveRequest(Entity),
    FocusEnterRequest(Entity),
}

#[derive(Resource, Reflect, Default)]
pub struct UiFocusState {
    pub mouse_focus: Option<Entity>,
    pub input_focus: Option<Entity>,
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct UiInputQuery {
    entity: Entity,
    ui_focus: &'static mut UiInput,
    interaction: Ref<'static, Interaction>,
    relative_cursor_position: Option<Ref<'static, RelativeCursorPosition>>,
    theme: Option<&'static mut ThemeComponent>,
    event_dispatcher: &'static UiInputEventDispatcher,
}

pub fn update_ui_input(
    mut query: Query<UiInputQuery>,
    mut commands: Commands,
    mut keyboard_event: MessageReader<KeyboardInput>,
    mouse_button_state: Res<ButtonInput<MouseButton>>,
    mut ui_focus_event: MessageReader<UiFocusEvent>,
    mut ui_focus_state: ResMut<UiFocusState>,
    mut wheel_event_cursor: Local<EventCursor<MouseWheel>>,
    mut button_event_cursor: Local<EventCursor<MouseButtonInput>>,
    mut wheel_events: Res<Events<MouseWheel>>,
    mut mouse_button_events: Res<Events<MouseButtonInput>>,
) {
    for UiInputQueryItem {
        entity,
        mut ui_focus,
        interaction,
        relative_cursor_position,
        event_dispatcher,
        ..
    } in &mut query
    {
        use UiInputEvent::*;

        if ui_focus.grab_mouse_when_focused
            && ui_focus.input_focused
            && *interaction == Interaction::None
        {
            for button in mouse_button_state.get_just_pressed() {
                event_dispatcher.send(MousePress(*button), &mut commands);
            }
            for button in mouse_button_state.get_just_released() {
                event_dispatcher.send(MouseRelease(*button), &mut commands);
            }
        }

        if *interaction != Interaction::None {
            let mut wheel_event_cursor_clone = wheel_event_cursor.clone();
            for event in wheel_event_cursor_clone.read(&wheel_events) {
                event_dispatcher.send(Wheel(event.clone()), &mut commands);
            }

            let mut button_event_cursor_clone = button_event_cursor.clone();
            for event in button_event_cursor_clone.read(&mouse_button_events) {
                event_dispatcher.send(RawMouseButton(event.clone()), &mut commands);
            }
        }

        if !interaction.is_changed()
            && !ui_focus.is_changed()
            && !relative_cursor_position
                .as_ref()
                .map(|r| r.is_changed())
                .unwrap_or_default()
        {
            continue;
        }

        if let Some(relative_cursor_position) = relative_cursor_position.as_ref() {
            if relative_cursor_position.is_changed() {
                if let Some(pos) = relative_cursor_position.normalized {
                    event_dispatcher.send(MouseMove(pos), &mut commands);
                }
            }
        }
        match (ui_focus.mouse_state, &*interaction) {
            (Interaction::Hovered | Interaction::None, Interaction::None) => {
                event_dispatcher.send(MouseLeave, &mut commands);
            }
            (Interaction::None, Interaction::Hovered | Interaction::Pressed) => {
                event_dispatcher.send(MouseEnter, &mut commands);
            }
            _ => {}
        };
        match (ui_focus.mouse_state, &*interaction) {
            (Interaction::Pressed, Interaction::None | Interaction::Hovered) => {
                for button in mouse_button_state.get_just_released() {
                    event_dispatcher.send(MouseRelease(*button), &mut commands);
                }
            }
            (Interaction::Hovered | Interaction::None, Interaction::Pressed) => {
                for button in mouse_button_state.get_just_pressed() {
                    event_dispatcher.send(MousePress(*button), &mut commands);
                }
            }
            _ => {}
        };
        ui_focus.set_mouse_state(*interaction);
    }
    for key in keyboard_event.read() {
        for UiInputQueryItem {
            entity,
            ui_focus,
            event_dispatcher,
            ..
        } in &mut query
        {
            if ui_focus.can_receive_keyboard_input() {
                event_dispatcher.send(UiInputEvent::KeyboardInput(key.clone()), &mut commands);
            }
        }
    }

    let set_theme_focused = |theme: Option<Mut<ThemeComponent>>, value: bool| {
        if let Some(mut theme) = theme {
            theme.set_flag(StyleFlags::FOCUSED, value);
        }
    };
    for event in ui_focus_event.read() {
        match event {
            UiFocusEvent::FocusLeaveRequest(e) => {
                if let Ok(UiInputQueryItem {
                    mut ui_focus,
                    theme,
                    event_dispatcher,
                    ..
                }) = query.get_mut(*e)
                {
                    ui_focus.input_focused = false;
                    event_dispatcher.send(UiInputEvent::KeyboardLeave, &mut commands);
                    set_theme_focused(theme, false);
                }
                ui_focus_state.input_focus = None;
            }
            UiFocusEvent::FocusEnterRequest(e) => {
                if let Some(UiInputQueryItem {
                    mut ui_focus,
                    theme,
                    event_dispatcher,
                    ..
                }) = ui_focus_state
                    .input_focus
                    .and_then(|old_node| query.get_mut(old_node).ok())
                {
                    ui_focus.input_focused = false;
                    event_dispatcher.send(UiInputEvent::KeyboardLeave, &mut commands);
                    set_theme_focused(theme, false);
                } else {
                    warn!(entity=?e, "can not release input focus of node");
                }
                if let Ok(UiInputQueryItem {
                    mut ui_focus,
                    theme,
                    event_dispatcher,
                    ..
                }) = query.get_mut(*e)
                {
                    ui_focus.input_focused = true;
                    event_dispatcher.send(UiInputEvent::KeyboardEnter, &mut commands);
                    set_theme_focused(theme, true);
                } else {
                    warn!(entity=?e, "can not enter input focus of node");
                }
                ui_focus_state.input_focus = Some(*e);
            }
        }
    }
    wheel_event_cursor.clear(&wheel_events);
    button_event_cursor.clear(&mouse_button_events);
}

pub type UiInputEventDispatcher = EventDispatcher<UiInputEvent>;
