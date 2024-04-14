use std::ops::Deref;

use crate::{
    make_bundle,
    prelude::*,
    theme::{StyleFlags, ThemeComponent},
};
use bevy::{ecs::query::QueryData, input::keyboard::KeyboardInput, ui::RelativeCursorPosition};
use bevy_relationship::reexport::SmallVec;
use bevy_trait_query::{queryable, ReadTraits};

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
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    pub struct UiInputEvent{
        pub receiver: Entity,
        pub node: Entity,
        pub kind: pub enum UiInputEventKind{
                MouseEnter,
                MouseLeave,
                MousePress(MouseButton),
                MouseRelease(MouseButton),
                KeybordEnter,
                KeyboardLeave,
                MouseMove(Vec2),
                KeyboardInput(KeyboardInput),
            }
    }
}

#[queryable]
pub trait UiInputDispatch {
    fn on_event(&self, event: UiInputEvent, commands: &mut Commands);
}

#[derive(Component, Debug, SmartDefault, Reflect)]
pub struct UiInput {
    #[reflect(ignore)]
    pub callbacks: SmallVec<[(Entity, SystemId<UiInputEvent>); 2]>,
    pub mouse_focused: bool,
    pub input_focused: bool,
    pub input_grabed: bool,
    #[default(true)]
    pub grab_mouse_when_focused: bool,
    pub self_interaction: Interaction,
    pub mouse_state: Interaction,
}

impl UiInput {
    pub fn can_receive_keyboard_input(&self) -> bool {
        self.input_focused || self.input_grabed
    }

    pub fn with_callback(mut self, receiver: Entity, systemid: SystemId<UiInputEvent>) -> Self {
        self.callbacks.push((receiver, systemid));
        self
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

#[derive(QueryData)]
#[query_data(mutable)]
pub struct UiInputQuery {
    entity: Entity,
    ui_focus: &'static mut UiInput,
    dispatch: Option<&'static dyn UiInputDispatch>,
    interaction: Ref<'static, Interaction>,
    relative_cursor_position: Option<Ref<'static, RelativeCursorPosition>>,
    theme: Option<&'static mut ThemeComponent>,
}

pub fn update_ui_input(
    mut query: Query<UiInputQuery>,
    mut commands: Commands,
    mut keyboard_event: EventReader<KeyboardInput>,
    mouse_button_state: Res<ButtonInput<MouseButton>>,
    mut ui_focus_event: EventReader<UiFocusEvent>,
    mut ui_focus_state: ResMut<UiFocusState>,
) {
    let mut run_callback =
        |kind: UiInputEventKind,
         entity: Entity,
         ui_focus: &UiInput,
         dispatch: &Option<ReadTraits<'_, dyn UiInputDispatch>>| {
            let event = UiInputEvent {
                receiver: entity,
                node: entity,
                kind: kind.clone(),
            };
            for (receiver, callback) in &ui_focus.callbacks {
                commands.run_system_with_input(
                    *callback,
                    UiInputEvent {
                        receiver: *receiver,
                        ..event.clone()
                    },
                );
            }
            if let Some(dispatch) = dispatch {
                for component in dispatch {
                    component.on_event(event.clone(), &mut commands);
                }
            }
        };
    for UiInputQueryItem {
        entity,
        mut ui_focus,
        dispatch,
        interaction,
        relative_cursor_position,
        ..
    } in &mut query
    {
        use UiInputEventKind::*;
        let mut call = |kind: UiInputEventKind| {
            run_callback(kind, entity, &ui_focus, &dispatch);
        };

        if ui_focus.grab_mouse_when_focused
            && ui_focus.input_focused
            && *interaction == Interaction::None
        {
            for button in mouse_button_state.get_just_pressed() {
                call(MousePress(*button));
            }
            for button in mouse_button_state.get_just_released() {
                call(MouseRelease(*button));
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
                    call(MouseMove(pos));
                }
            }
        }
        match (ui_focus.mouse_state, &*interaction) {
            (Interaction::Hovered | Interaction::None, Interaction::None) => {
                call(MouseLeave);
            }
            (Interaction::None, Interaction::Hovered | Interaction::Pressed) => {
                call(MouseEnter);
            }
            _ => {}
        };
        match (ui_focus.mouse_state, &*interaction) {
            (Interaction::Pressed, Interaction::None | Interaction::Hovered) => {
                for button in mouse_button_state.get_just_released() {
                    call(MouseRelease(*button));
                }
            }
            (Interaction::Hovered | Interaction::None, Interaction::Pressed) => {
                for button in mouse_button_state.get_just_pressed() {
                    call(MousePress(*button));
                }
            }
            _ => {}
        };
        ui_focus.mouse_state = *interaction;
    }
    for key in keyboard_event.read() {
        for UiInputQueryItem {
            entity,
            ui_focus,
            dispatch,
            ..
        } in &mut query
        {
            if ui_focus.can_receive_keyboard_input() {
                run_callback(
                    UiInputEventKind::KeyboardInput(key.clone()),
                    entity,
                    &ui_focus,
                    &dispatch,
                );
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
                    dispatch,
                    theme,
                    ..
                }) = query.get_mut(*e)
                {
                    ui_focus.input_focused = false;
                    run_callback(UiInputEventKind::KeyboardLeave, *e, &ui_focus, &dispatch);
                    set_theme_focused(theme, false);
                }
                ui_focus_state.input_focus = None;
            }
            UiFocusEvent::FocusEnterRequest(e) => {
                if let Some(UiInputQueryItem {
                    mut ui_focus,
                    dispatch,
                    theme,
                    ..
                }) = ui_focus_state
                    .input_focus
                    .and_then(|old_node| query.get_mut(old_node).ok())
                {
                    ui_focus.input_focused = false;
                    run_callback(UiInputEventKind::KeyboardLeave, *e, &ui_focus, &dispatch);
                    set_theme_focused(theme, false);
                } else {
                    warn!(entity=?e, "can not release input focus of node");
                }
                if let Ok(UiInputQueryItem {
                    mut ui_focus,
                    dispatch,
                    theme,
                    ..
                }) = query.get_mut(*e)
                {
                    ui_focus.input_focused = true;
                    run_callback(UiInputEventKind::KeybordEnter, *e, &ui_focus, &dispatch);
                    set_theme_focused(theme, true);
                } else {
                    warn!(entity=?e, "can not enter input focus of node");
                }
                ui_focus_state.input_focus = Some(*e);
            }
        }
    }
}

make_bundle! {
    @from input: UiInput,
    @addon UiInputExt,
    UiInputBundle {
        pub input: UiInput,
        pub interaction: Interaction,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
        pub relative_cursor_position: RelativeCursorPosition,
    }
}
