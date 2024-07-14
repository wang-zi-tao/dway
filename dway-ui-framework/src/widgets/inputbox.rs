use std::cmp::Ordering;

use bevy::{
    ecs::system::RunSystemOnce,
    input::keyboard::Key,
    text::{BreakLineOn, TextLayoutInfo},
    ui::RelativeCursorPosition,
};
use bevy_trait_query::RegisterExt;

use crate::{
    prelude::*,
    theme::{ThemeComponent, WidgetKind},
};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    pub struct UiInputboxEvent{
        pub receiver: Entity,
        pub widget: Entity,
        pub kind:
        enum UiInputboxEventKind {
            Enter,
            Changed,
            CursorMoved,
        }
    }
}

#[derive(Debug)]
pub enum UiInputCommand {
    Insert(usize, String),
    Delete(usize, String),
    Replace(usize, String, String),
}
impl UiInputCommand {
    pub fn new_delete(input_box_state: &UiInputBoxState) -> Self {
        let cursor = input_box_state.cursor_char_index();
        let char = input_box_state
            .data()
            .char_indices()
            .find_map(|(pos, c)| (pos == *cursor).then(|| c.to_string()));
        Self::Delete(
            *input_box_state.cursor_char_index(),
            char.unwrap_or_default(),
        )
    }

    pub fn apply(&self, input_box_state: &mut UiInputBoxState) {
        match self {
            UiInputCommand::Insert(p, d) => {
                input_box_state.data_mut().insert_str(*p, d);
            }
            UiInputCommand::Delete(p, d) => {
                let split_off = input_box_state.data_mut().split_off(*p);
                input_box_state
                    .data_mut()
                    .push_str(split_off.split_at(d.len()).1);
            }
            UiInputCommand::Replace(p, remove, insert) => {
                let split_off = input_box_state.data_mut().split_off(*p);
                input_box_state.data_mut().push_str(insert);
                input_box_state
                    .data_mut()
                    .push_str(split_off.split_at(remove.len()).1);
            }
        }
    }
}

structstruck::strike! {
    #[derive(Component, SmartDefault)]
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    pub struct UiInputBox{
        pub placeholder: String,
        #[reflect(ignore)]
        pub callback: CallbackSlots<UiInputboxEvent>,
        pub kind:
            #[derive(Default)]
            enum UiInputBoxKind{
                #[default]
                Normal,
                Password,
                Path,
            },
        pub readonly: bool,
        pub multi_line: bool,
        pub storage_key: Option<String>,
        #[default(24.0)]
        pub text_size: f32,
        pub font: Option<Handle<Font>>,
    }
}

impl UiInputBox {
    pub fn register_callback(&mut self, callback: Callback<UiInputboxEvent>) {
        self.callback.push(callback);
    }
}

pub fn move_cursor(
    position: Vec2,
    inputbox: &UiInputBox,
    text_layout: &TextLayoutInfo,
    inputbox_state: &mut UiInputBoxState,
) {
    let line_start = position.y - position.y % inputbox.text_size;
    let line_end = line_start + inputbox.text_size;
    let glyphs = &text_layout.glyphs;
    if let Some((index, glyph)) = glyphs
        .binary_search_by(|glyph| {
            if glyph.position.y > line_end {
                return Ordering::Greater;
            }
            if glyph.position.y < line_start {
                return Ordering::Less;
            }
            if glyph.position.x - 0.5 * glyph.size.x > position.x {
                return Ordering::Greater;
            }
            if glyph.position.x + 0.5 * glyph.size.x < position.x {
                return Ordering::Less;
            }
            Ordering::Equal
        })
        .map(|index| (index, &glyphs[index]))
        .ok()
        .map(|(index, glyph)| {
            if position.x > glyph.position.x {
                if let Some(glyph) = glyphs.get(index + 1) {
                    return (index + 1, glyph);
                }
            }
            (index, glyph)
        })
        .or_else(|| {
            glyphs
                .last()
                .map(|last| (glyphs.len().saturating_sub(1), last))
        })
    {
        let mut byte_index = glyph.byte_index;
        if glyphs.len() == index + 1 && position.x > glyph.position.x
            || line_start > glyph.position.y
        {
            byte_index = inputbox_state
                .data()
                .floor_char_boundary(byte_index.saturating_add(1));
        }
        inputbox_state.set_cursor_char_index(byte_index);
    };
}

fn on_input_event(
    In(event): In<UiInputEvent>,
    mut query: Query<(
        Entity,
        Ref<Interaction>,
        &UiInputBoxWidget,
        &UiInputBox,
        &mut UiInputBoxState,
        &mut UiInput,
        &RelativeCursorPosition,
    )>,
    text_node_query: Query<(Ref<Node>, Ref<TextLayoutInfo>)>,
    mut input_focus_event: EventWriter<UiFocusEvent>,
    mut commands: Commands,
) {
    let Ok((
        entity,
        interaction,
        inputbox_widget,
        inputbox,
        mut inputbox_state,
        focus_state,
        relative_pos,
    )) = query.get_mut(event.node)
    else {
        return;
    };

    match event.kind {
        UiInputEventKind::MousePress(_) => {
            if *interaction == Interaction::None {
                input_focus_event.send(UiFocusEvent::FocusLeaveRequest(entity));
                return;
            }

            if !focus_state.can_receive_keyboard_input() {
                input_focus_event.send(UiFocusEvent::FocusEnterRequest(entity));
                return;
            }

            let Ok((node, text_layout)) = text_node_query.get(inputbox_widget.node_text_entity)
            else {
                warn!(entity=?entity, "the UiInputBox has broken");
                return;
            };

            if relative_pos.mouse_over() {
                if let Some(normalized) = relative_pos.normalized {
                    move_cursor(
                        normalized * node.size(),
                        inputbox,
                        &text_layout,
                        &mut inputbox_state,
                    );
                }
            }
        }
        UiInputEventKind::MouseMove(_) => {
            if *interaction == Interaction::Pressed {
                if let Some(normalized) = relative_pos.normalized {
                    let Ok((node, text_layout)) =
                        text_node_query.get(inputbox_widget.node_text_entity)
                    else {
                        warn!(entity=?entity, "the UiInputBox has broken");
                        return;
                    };

                    move_cursor(
                        normalized * node.size(),
                        inputbox,
                        &text_layout,
                        &mut inputbox_state,
                    );
                }
            }
        }
        UiInputEventKind::KeyboardInput(key) => {
            let mut run_callbacks = |event: UiInputboxEventKind| {
                for (receiver, callback) in &inputbox.callback {
                    commands.run_system_with_input(
                        *callback,
                        UiInputboxEvent {
                            receiver: *receiver,
                            widget: entity,
                            kind: event.clone(),
                        },
                    );
                }
            };

            if key.state.is_pressed() {
                return;
            }

            let Ok((_, text_layout)) = text_node_query.get(inputbox_widget.node_text_entity) else {
                warn!(entity=?entity, "the UiInputBox has broken");
                return;
            };

            match &key.logical_key {
                Key::Character(s) => {
                    if !inputbox.readonly {
                        UiInputCommand::Insert(*inputbox_state.cursor_char_index(), s.to_string())
                            .apply(&mut inputbox_state);
                        let position = inputbox_state
                            .data()
                            .floor_char_boundary(inputbox_state.cursor_char_index() + s.len());
                        inputbox_state.set_cursor_char_index(position);
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                        run_callbacks(UiInputboxEventKind::Changed);
                    }
                }
                Key::Space => {
                    if !inputbox.readonly {
                        UiInputCommand::Insert(
                            *inputbox_state.cursor_char_index(),
                            " ".to_string(),
                        )
                        .apply(&mut inputbox_state);
                        let position = inputbox_state
                            .data()
                            .floor_char_boundary(inputbox_state.cursor_char_index() + " ".len());
                        inputbox_state.set_cursor_char_index(position);
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                        run_callbacks(UiInputboxEventKind::Changed);
                    }
                }
                Key::Enter => {
                    if !inputbox.readonly && inputbox.multi_line {
                        UiInputCommand::Insert(
                            *inputbox_state.cursor_char_index(),
                            "\n".to_string(),
                        )
                        .apply(&mut inputbox_state);
                        let position = inputbox_state
                            .data()
                            .floor_char_boundary(inputbox_state.cursor_char_index() + "\n".len());
                        inputbox_state.set_cursor_char_index(position);
                    }
                    if !inputbox.readonly {
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                        run_callbacks(UiInputboxEventKind::Enter);
                    }
                }
                Key::Backspace => {
                    if !inputbox.readonly && *inputbox_state.cursor_char_index() != 0 {
                        *inputbox_state.cursor_char_index_mut() =
                            inputbox_state.data().floor_char_boundary(
                                inputbox_state.cursor_char_index().saturating_sub(1),
                            );
                        UiInputCommand::new_delete(&inputbox_state).apply(&mut inputbox_state);
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                        run_callbacks(UiInputboxEventKind::Changed);
                    }
                }
                Key::Delete => {
                    if !inputbox.readonly {
                        UiInputCommand::new_delete(&inputbox_state).apply(&mut inputbox_state);
                        run_callbacks(UiInputboxEventKind::Changed);
                    }
                }
                Key::Home => {
                    *inputbox_state.cursor_char_index_mut() = 0;
                    run_callbacks(UiInputboxEventKind::CursorMoved);
                }
                Key::End => {
                    *inputbox_state.cursor_char_index_mut() = inputbox_state.data().len();
                    run_callbacks(UiInputboxEventKind::CursorMoved);
                }
                Key::ArrowRight => {
                    *inputbox_state.cursor_char_index_mut() = inputbox_state
                        .data()
                        .floor_char_boundary(inputbox_state.cursor_char_index().saturating_add(1));
                    run_callbacks(UiInputboxEventKind::CursorMoved);
                }
                Key::ArrowLeft => {
                    *inputbox_state.cursor_char_index_mut() = inputbox_state
                        .data()
                        .ceil_char_boundary(inputbox_state.cursor_char_index().saturating_sub(1));
                    run_callbacks(UiInputboxEventKind::CursorMoved);
                }
                Key::ArrowUp => {
                    move_cursor(
                        *inputbox_state.cursor_position() - inputbox.text_size * Vec2::Y,
                        inputbox,
                        &text_layout,
                        &mut inputbox_state,
                    );
                }
                Key::ArrowDown => {
                    move_cursor(
                        *inputbox_state.cursor_position() + inputbox.text_size * Vec2::Y,
                        inputbox,
                        &text_layout,
                        &mut inputbox_state,
                    );
                }
                Key::Escape => {
                    input_focus_event.send(UiFocusEvent::FocusLeaveRequest(entity));
                }
                _ => {}
            };
        }
        _ => (),
    }
}

impl UiInputDispatch for UiInputBox {
    fn on_event(&self, event: UiInputEvent, commands: &mut Commands) {
        commands.add(|world: &mut World| {
            world.run_system_once_with(event, on_input_event);
        });
    }
}

dway_widget! {
UiInputBox=>
@plugin{
    app.register_type::<UiInputBox>();
    app.register_component_as::<dyn UiInputDispatch, UiInputBox>();
    app.register_callback(on_input_event);
}
@state_reflect()
@global(theme: Theme)
@use_state(pub data: String)
@use_state(pub cursor_char_index: usize)
@use_state(pub show_cursor: bool = true)
@use_state(pub cursor_position: Vec2)
@use_state(#[reflect(ignore)] pub undo: undo::history::History<UiInputCommand>)
@bundle{{
    pub ui_focus: UiInput,
    pub relative_cursor_position: RelativeCursorPosition,
    pub interaction: Interaction,
    pub theme: ThemeComponent = ThemeComponent::widget(WidgetKind::Inputbox),
}}
@world_query(focus_policy: &mut FocusPolicy)
@arg(text_node_query: Query<Ref<TextLayoutInfo>>)
@before{{
    if !widget.inited {
        *focus_policy = FocusPolicy::Block;
    }
    if !widget.inited && !prop.readonly{
        state.set_cursor_position(Vec2::ZERO);
    }

    if let Ok(text_layout) = text_node_query.get(widget.node_text_entity) {
        if text_layout.is_changed() || state.cursor_char_index_is_changed() {
            let byte_index = *state.cursor_char_index();
            let glyphs = &text_layout.glyphs;
            let position = if let Ok(glyph_index) = glyphs
                .as_slice()
                .binary_search_by_key(&byte_index, |glyph| glyph.byte_index)
            {
                let glyph = &glyphs[glyph_index];
                Vec2::new(
                    glyph.position.x - 0.5 * glyph.size.x,
                    glyph.position.y - glyph.position.y % prop.text_size,
                )
            } else if byte_index == state.data().len() {
                glyphs.last().map(|glyph|{ Vec2::new(
                    glyph.position.x + 0.5 * glyph.size.x,
                    glyph.position.y - glyph.position.y % prop.text_size,
                )}).unwrap_or_default()
            } else {
                Vec2::ZERO
            };
            state.set_cursor_position(position);
        }
    }
}}
<UiTextBundle @id="text" @style="full" Text=(Text{
    sections: {
        if state.data().is_empty() {
            vec![TextSection{
                value: prop.placeholder.clone(),
                style: TextStyle {
                    font: prop.font.clone().unwrap_or_else(||theme.default_font()),
                    font_size: prop.text_size,
                    color: theme.color("inputbox:placeholder"),
                },
            }]
        } else {
            vec![ TextSection{
                value: state.data().clone(),
                style: TextStyle {
                    font: prop.font.clone().unwrap_or_else(||theme.default_font()),
                    font_size: prop.text_size,
                    color: theme.color("inputbox:text"),
                },
            } ]
        }
    },
    justify: JustifyText::Left,
    linebreak_behavior: BreakLineOn::AnyCharacter,
}) >
</>
<MiniNodeBundle @style="absolute full" @if(*state.show_cursor())>
    <MiniNodeBundle @id="cursor" Style=(Style{
        left: Val::Px(state.cursor_position().x),
        top: Val::Px(state.cursor_position().y),
        height: Val::Px(prop.text_size),
        ..style!("w-2")
    })
    @material(RoundedUiRectMaterial=>rounded_rect(theme.color("inputbox:cursor"), 4.0)) />
</MiniNodeBundle>
}
