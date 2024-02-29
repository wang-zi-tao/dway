use std::{cmp::Ordering, ops::Range};

use bevy::{
    input::{
        keyboard::{Key, KeyboardInput},
        mouse::MouseButtonInput,
        ButtonState,
    },
    text::{
        scale_value, BreakLineOn, FontAtlasSets, Text2dBounds, TextLayoutInfo, TextPipeline,
        TextSettings, YAxisOrientation,
    },
    ui::RelativeCursorPosition,
    utils::HashSet,
    window::{PrimaryWindow, WindowScaleFactorChanged},
};
use bevy_relationship::reexport::SmallVec;

use crate::prelude::*;

use super::text::UiTextBundle;

structstruck::strike! {
    pub struct UiInputboxEvent{
        pub receiver: Entity,
        pub widget: Entity,
        pub kind:
        #[derive(Clone, Copy, PartialEq, Eq)]
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
                input_box_state.data_mut().insert_str(*p, &d);
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
    #[derive(Debug, Component, SmartDefault)]
    pub struct UiInputBox{
        pub placeholder: String,
        pub callback: CallbackSlots<UiInputboxEvent>,
        pub kind:
            #[derive(Debug, Clone, Default)]
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

pub fn move_cursor(position: Vec2, inputbox: &UiInputBox, inputbox_state: &mut UiInputBoxState) {
    let line_start = position.y - position.y % inputbox.text_size;
    let line_end = line_start + inputbox.text_size;
    let glyphs = &inputbox_state.test_layout().glyphs;
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
        let mut position = glyph.position - 0.5 * glyph.size;
        if glyphs.len() == index + 1 && position.x > glyph.position.x
            || line_start > glyph.position.y
        {
            byte_index = inputbox_state
                .data()
                .floor_char_boundary(byte_index.saturating_add(1));
            position.x += glyph.size.x;
        }
        position.y = position.y - position.y % inputbox.text_size;
        inputbox_state.set_cursor_char_index(byte_index);
        inputbox_state.set_cursor_position(position);
    };
}

pub fn process_ui_inputbox_event(
    mut keyboard_event: EventReader<KeyboardInput>,
    mut mouse_input: EventReader<MouseButtonInput>,
    mut query: Query<(
        Entity,
        Ref<Node>,
        &UiInputBox,
        &mut UiInputBoxState,
        &mut UiInput,
        &RelativeCursorPosition,
    )>,
    theme: Res<Theme>,
    mut queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    text_settings: Res<TextSettings>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_sets: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut mouse_focus_event: EventWriter<UiFocusEvent>,
    mut commands: Commands,
) {
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    let mouse_events = mouse_input.read().collect::<SmallVec<[_; 4]>>();
    let key_events = keyboard_event.read().collect::<SmallVec<[_; 4]>>();
    for (entity, node, inputbox, mut inputbox_state, focus_state, relative_pos) in &mut query {
        let mut run_callbacks = |event: UiInputboxEventKind| {
            for (receiver, callback) in &inputbox.callback {
                commands.run_system_with_input(
                    *callback,
                    UiInputboxEvent {
                        receiver: *receiver,
                        widget: entity,
                        kind: event,
                    },
                );
            }
        };
        if focus_state.can_receive_keyboard_input() {
            for key in &key_events {
                if key.state.is_pressed() {
                    continue;
                }
                match &key.logical_key {
                    Key::Character(s) => {
                        if !inputbox.readonly {
                            UiInputCommand::Insert(
                                *inputbox_state.cursor_char_index(),
                                s.to_string(),
                            )
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
                            let position = inputbox_state.data().floor_char_boundary(
                                inputbox_state.cursor_char_index() + " ".len(),
                            );
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
                            let position = inputbox_state.data().floor_char_boundary(
                                inputbox_state.cursor_char_index() + "\n".len(),
                            );
                            inputbox_state.set_cursor_char_index(position);
                        }
                        if !inputbox.readonly {
                            run_callbacks(UiInputboxEventKind::CursorMoved);
                            run_callbacks(UiInputboxEventKind::Enter);
                        }
                    }
                    Key::Backspace => {
                        if !inputbox.readonly {
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
                        *inputbox_state.cursor_char_index_mut() =
                            inputbox_state.data().floor_char_boundary(
                                inputbox_state.cursor_char_index().saturating_add(1),
                            );
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                    }
                    Key::ArrowLeft => {
                        *inputbox_state.cursor_char_index_mut() =
                            inputbox_state.data().ceil_char_boundary(
                                inputbox_state.cursor_char_index().saturating_sub(1),
                            );
                        run_callbacks(UiInputboxEventKind::CursorMoved);
                    }
                    Key::ArrowUp => {
                        move_cursor(
                            *inputbox_state.cursor_position() - inputbox.text_size * Vec2::Y,
                            &inputbox,
                            &mut inputbox_state,
                        );
                    }
                    Key::ArrowDown => {
                        move_cursor(
                            *inputbox_state.cursor_position() + inputbox.text_size * Vec2::Y,
                            &inputbox,
                            &mut inputbox_state,
                        );
                    }
                    _ => {}
                };
            }
        }

        if inputbox_state.cursor_char_index_is_changed()
            || inputbox_state.data_is_changed()
            || node.is_changed()
            || queue.remove(&entity)
        {
            let text = {
                let value = inputbox_state.data();
                let style = TextStyle {
                    font: inputbox
                        .font
                        .clone()
                        .unwrap_or_else(|| theme.default_font()),
                    font_size: inputbox.text_size,
                    color: theme.color("inputbox:text"),
                };
                Text {
                    sections: vec![TextSection::new(value, style)],
                    justify: JustifyText::Left,
                    linebreak_behavior: BreakLineOn::AnyCharacter,
                    ..default()
                }
            };
            let text_bounds = Vec2::new(
                if text.linebreak_behavior == BreakLineOn::NoWrap {
                    f32::INFINITY
                } else {
                    scale_value(node.size().x, scale_factor)
                },
                scale_value(node.size().y, scale_factor),
            );
            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.justify,
                text.linebreak_behavior,
                text_bounds,
                &mut font_atlas_sets,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                YAxisOrientation::TopToBottom,
            ) {
                Ok(text_layout) => {
                    inputbox_state.set_test_layout(text_layout);
                }
                Err(TextError::NoSuchFont) => {
                    queue.insert(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
            };
        }

        if inputbox_state.test_layout_is_changed() || inputbox_state.cursor_char_index_is_changed()
        {
            let byte_index = *inputbox_state.cursor_char_index();
            let glyphs = &inputbox_state.test_layout().glyphs;
            if let Ok(glyph_index) = glyphs
                .as_slice()
                .binary_search_by_key(&byte_index, |glyph| glyph.byte_index)
            {
                let glyph = &glyphs[glyph_index];
                let position = Vec2::new(
                    glyph.position.x,
                    glyph.position.y - glyph.position.y % inputbox.text_size,
                );
                inputbox_state.set_cursor_position(position);
            } else if byte_index == inputbox_state.data().len() {
                if let Some(glyph) = glyphs.last() {
                    let position = Vec2::new(
                        glyph.position.x + 0.5 * glyph.size.x,
                        glyph.position.y - glyph.position.y % inputbox.text_size,
                    );
                    inputbox_state.set_cursor_position(position);
                }
            }
        }
    }
    for mouse_event in &mouse_events {
        if mouse_event.state.is_pressed() {
            for (entity, node, inputbox, mut inputbox_state, focus_state, relative_pos) in
                &mut query
            {
                if let Some(normalized) = relative_pos.normalized {
                    if relative_pos.mouse_over() {
                        mouse_focus_event.send(UiFocusEvent::FocusEnterRequest(entity));
                        move_cursor(normalized * node.size(), &inputbox, &mut inputbox_state);
                    }
                }
            }
        }
    }
}

dway_widget! {
UiInputBox=>
@global(theme: Theme)
@use_state(pub data: String)
@use_state(pub cursor_char_index: usize)
@use_state(pub show_cursor: bool = true)
@use_state(pub cursor_position: Vec2)
@use_state(pub undo: undo::history::History<UiInputCommand>)
@use_state(pub test_layout: TextLayoutInfo)
@bundle{{
    pub ui_focus: UiInput,
    pub relative_cursor_position: RelativeCursorPosition,
    pub interaction: Interaction,
}}
@before{{
    if !widget.inited && !prop.readonly{
        state.set_cursor_position(Vec2::ZERO);
    }
}}
<UiTextBundle @id="text" Text=(Text{
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
}) />
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
