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
        enum UiInputEventKind {
            Enter,
            Changed,
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
    pub fn new_delete(input_box_state: &UiInputBoxState, cursor_move: isize) -> Self {
        let cursor = input_box_state.cursor_char_index();
        // let char = input_box_state.data().get(cursor);
        todo!()
    }

    pub fn apply(&self, input_box_state: &mut UiInputBoxState) {
        match self {
            UiInputCommand::Insert(p, d) => {
                input_box_state.data_mut().insert_str(*p, &d);
            }
            UiInputCommand::Delete(p, d) => {}
            UiInputCommand::Replace(_, _, _) => todo!(),
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
    }
}

impl UiInputBox {
    pub fn register_callback(&mut self, callback: Callback<UiInputboxEvent>) {
        self.callback.push(callback);
    }
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
) {
    let scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.0);

    let mouse_events = mouse_input.read().collect::<SmallVec<[_; 4]>>();
    let key_events = keyboard_event.read().collect::<SmallVec<[_; 4]>>();
    for (entity, node, inputbox, mut inputbox_state, focus_state, relative_pos) in &mut query {
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
                                .ceil_char_boundary(inputbox_state.data().len().saturating_sub(1));
                            inputbox_state.set_cursor_char_index(position);
                        }
                    }
                    Key::Backspace => {
                        if !inputbox.readonly {
                            UiInputCommand::new_delete(&inputbox_state, -1)
                                .apply(&mut inputbox_state);
                        }
                    }
                    Key::Delete => {
                        if !inputbox.readonly {
                            UiInputCommand::new_delete(&inputbox_state, 1)
                                .apply(&mut inputbox_state);
                        }
                    }
                    Key::Home => {
                        *inputbox_state.cursor_char_index_mut() = 0;
                    }
                    Key::End => {
                        *inputbox_state.cursor_char_index_mut() = inputbox_state.data().len();
                    }
                    Key::ArrowRight => {
                        *inputbox_state.cursor_char_index_mut() = inputbox_state
                            .cursor_char_index()
                            .saturating_add(1)
                            .min(inputbox_state.data().len());
                    }
                    Key::ArrowLeft => {
                        *inputbox_state.cursor_char_index_mut() =
                            inputbox_state.cursor_char_index().saturating_sub(1);
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
                    font: theme.default_font(),
                    font_size: 24.0,
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
                YAxisOrientation::BottomToTop,
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
                let position = glyph.position - Vec2::new(glyph.size.x, 0.0);
                inputbox_state.set_cursor_position(position);
            }
        }

        for mouse_event in &mouse_events {
            if mouse_event.state.is_pressed() {
                if let Some(position) = relative_pos.normalized {
                    if !focus_state.can_receive_keyboard_input() {
                        mouse_focus_event.send(UiFocusEvent::FocusEnterRequest(entity));
                    }
                    let glyphs = &inputbox_state.test_layout().glyphs;
                    if let Ok(glyph_index) = glyphs.binary_search_by(|glyph| {
                        if glyph.position.y > position.y {
                            return Ordering::Greater;
                        } else if glyph.position.y + glyph.size.y < position.y {
                            return Ordering::Less;
                        }
                        glyph
                            .position
                            .x
                            .partial_cmp(&position.x)
                            .unwrap_or(Ordering::Less)
                    }) {
                        let glyph = &glyphs[glyph_index];
                        let byte_index = glyph.byte_index;
                        let position = glyph.position - Vec2::new(glyph.size.x, 0.0);
                        inputbox_state.set_cursor_char_index(byte_index);
                        inputbox_state.set_cursor_position(position);
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
                    font: theme.default_font(),
                    font_size: 24.0,
                    color: theme.color("inputbox:placeholder"),
                },
            }]
        } else {
            vec![ TextSection{
                value: state.data().clone(),
                style: TextStyle {
                    font: theme.default_font(),
                    font_size: 24.0,
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
        ..style!("w-2 h-24")
    })
    @material(RoundedUiRectMaterial=>rounded_rect(theme.color("inputbox:cursor"), 4.0)) />
</MiniNodeBundle>
}
