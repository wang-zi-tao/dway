use std::cmp::Ordering;

use bevy::{
    ecs::{component::{ComponentId, HookContext}, world::DeferredWorld},
    input::keyboard::Key,
    text::TextLayoutInfo,
    ui::RelativeCursorPosition,
};
use unicode_segmentation::{GraphemeIndices, UnicodeSegmentation};

use super::textarea::UiTextArea;
use crate::{impl_event_receiver, prelude::*};

#[derive(Component, SmartDefault, Reflect)]
#[require(UiInput, RelativeCursorPosition)]
#[component(on_insert=on_insert_text_cursor)]
#[component(on_replace=on_replace_text_cursor)]
pub struct UiTextCursor {
    pub byte_index: usize,
    #[default(true)]
    pub show_corsor: bool,
    pub position: Vec2,
    pub cursor_width: f32,
    #[default(color!("#0000ff"))]
    pub corsor_color: Color,
    pub line_height: f32,
    #[default(Entity::PLACEHOLDER)]
    pub cursor_entity: Entity,
}

pub fn on_insert_text_cursor(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let textarea = world.get_mut::<UiTextArea>(entity).unwrap();
    let font_size = textarea.font_size;
    let mut textcursor = world.get_mut::<UiTextCursor>(entity).unwrap();
    let line_height = font_size * 1.2;
    textcursor.line_height = line_height;
    let color = textcursor.corsor_color;

    if textcursor.cursor_entity == Entity::PLACEHOLDER {
        let cursor_entity = world
            .commands()
            .spawn((
                Node {
                    width: Val::Px(2.0),
                    height: Val::Px(line_height),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..Default::default()
                },
                BackgroundColor(color),
            ))
            .set_parent(entity)
            .id();

        let mut textcursor = world.get_mut::<UiTextCursor>(entity).unwrap();
        textcursor.cursor_entity = cursor_entity;
    }
}

pub fn on_replace_text_cursor(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let textcursor = world.get::<UiTextCursor>(entity).unwrap();
    let cursor_entity = textcursor.cursor_entity;
    world.commands().queue(move |world: &mut World| {
        if let Ok(entity_mut) = world.get_entity_mut(cursor_entity) {
            entity_mut.despawn_recursive();
        }
    });
}

impl UiTextCursor {
    pub fn get_cursor_position_of_glyph(
        &self,
        gr_index: Option<usize>,
        text_layout: &TextLayoutInfo,
    ) -> Vec2 {
        let glyphs = &text_layout.glyphs;
        if let Some(glyph) = gr_index.and_then(|i| glyphs.get(i)) {
            Vec2::new(
                glyph.position.x - 0.5 * glyph.size.x,
                glyph.position.y - glyph.position.y % self.line_height + self.line_height * 0.5,
            )
        } else {
            glyphs
                .last()
                .map(|glyph| {
                    Vec2::new(
                        glyph.position.x + 0.5 * glyph.size.x,
                        glyph.position.y - glyph.position.y % self.line_height
                            + self.line_height * 0.5,
                    )
                })
                .unwrap_or_else(|| Vec2::new(0.0, self.line_height * 0.5))
        }
    }

    pub fn position_to_glyph_index(&self, offset: Vec2, text_layout: &TextLayoutInfo) -> usize {
        let line_start = offset.y - offset.y % self.line_height;
        let line_end = line_start + self.line_height;
        let glyphs = &text_layout.glyphs;
        let mut glyph_index = glyphs
            .binary_search_by(|glyph| {
                if glyph.position.y > line_end {
                    return Ordering::Greater;
                }
                if glyph.position.y < line_start {
                    return Ordering::Less;
                }
                if glyph.position.x - 0.5 * glyph.size.x > offset.x {
                    return Ordering::Greater;
                }
                if glyph.position.x + 0.5 * glyph.size.x < offset.x {
                    return Ordering::Less;
                }
                Ordering::Equal
            })
            .unwrap_or_else(|index| index);
        if let Some(glyph) = glyphs.get(glyph_index) {
            if offset.x > glyph.position.x {
                glyph_index += 1;
            }
            glyph_index.min(glyphs.len())
        } else {
            glyphs.len()
        }
    }

    pub fn glyph_index_to_byte_index(&self, textarea: &UiTextArea, index: usize) -> usize {
        UnicodeSegmentation::grapheme_indices(&*textarea.data, true)
            .filter(|(_, c)| *c != "\n")
            .nth(index)
            .map(|(i, _)| i)
            .unwrap_or_else(|| textarea.data.len())
    }

    pub fn byte_index_to_glyph_index(
        &self,
        textarea: &UiTextArea,
        byte_index: usize,
    ) -> Option<usize> {
        UnicodeSegmentation::grapheme_indices(&*textarea.data, true)
            .filter(|(_, c)| *c != "\n")
            .position(|(index, value)| index + value.len() > byte_index)
    }

    pub fn set_glyph_index(&mut self, textarea: &UiTextArea, index: usize) {
        let byte_index = self.glyph_index_to_byte_index(textarea, index);
        self.byte_index = byte_index;
    }

    pub fn set_byte_index(&mut self, index: usize) {
        self.byte_index = index;
    }

    pub fn create_change_event(&self) -> UiTextCursorEvent {
        UiTextCursorEvent::ChangePosition {
            position: self.position,
            byte_index: self.byte_index,
        }
    }
}

pub fn text_cursor_on_input_system(
    event: UiEvent<UiInputEvent>,
    mut query: Query<(
        Entity,
        &UiTextArea,
        &mut UiTextCursor,
        Option<&EventDispatcher<UiTextCursorEvent>>,
        &Interaction,
        &mut UiInput,
        &RelativeCursorPosition,
        &ComputedNode,
    )>,
    text_query: Query<&TextLayoutInfo>,
    mut input_focus_event: EventWriter<UiFocusEvent>,
    mut commands: Commands,
) {
    let Ok((
        entity,
        textarea,
        mut cursor,
        event_dispatcher,
        interaction,
        ui_input,
        relative_pos,
        computed_node,
    )) = query.get_mut(event.sender())
    else {
        return;
    };

    let Ok(text_layout) = text_query.get(textarea.text_entity) else {
        return;
    };

    match &*event {
        UiInputEvent::MousePress(_) => {
            if !ui_input.can_receive_keyboard_input() {
                input_focus_event.send(UiFocusEvent::FocusEnterRequest(entity));
            }

            if let Some(normalized) = relative_pos.normalized {
                let glyph_index =
                    cursor.position_to_glyph_index(normalized * computed_node.size(), text_layout);
                cursor.set_glyph_index(textarea, glyph_index);
            }
        }
        UiInputEvent::MouseMove(_) => {
            if *interaction == Interaction::Pressed {
                if let Some(normalized) = relative_pos.normalized {
                    let glyph_index = cursor
                        .position_to_glyph_index(normalized * computed_node.size(), text_layout);
                    cursor.set_glyph_index(textarea, glyph_index);
                }
            }
        }
        UiInputEvent::KeyboardInput(key) => {
            if key.state.is_pressed() {
                return;
            }
            match key.logical_key {
                Key::Home => {
                    cursor.set_byte_index(0);
                }

                Key::End => {
                    let byte_index = textarea.data.len();
                    cursor.set_byte_index(byte_index);
                }
                Key::ArrowRight => {
                    let byte_index = textarea
                        .data
                        .ceil_char_boundary(cursor.byte_index.saturating_add(1));
                    cursor.set_byte_index(byte_index);
                }
                Key::ArrowLeft => {
                    let byte_index = textarea
                        .data
                        .ceil_char_boundary(cursor.byte_index.saturating_sub(1));
                    cursor.set_byte_index(byte_index);
                }
                Key::ArrowUp => {
                    let glyph_index = cursor.position_to_glyph_index(
                        cursor.position - cursor.line_height * Vec2::Y,
                        text_layout,
                    );
                    cursor.set_glyph_index(textarea, glyph_index);
                }
                Key::ArrowDown => {
                    let glyph_index = cursor.position_to_glyph_index(
                        cursor.position + cursor.line_height * Vec2::Y,
                        text_layout,
                    );
                    cursor.set_glyph_index(textarea, glyph_index);
                }
                Key::Escape => {
                    input_focus_event.send(UiFocusEvent::FocusLeaveRequest(entity));
                }
                _ => {}
            }
        }
        _ => {}
    }
    if cursor.is_changed() {
        EventDispatcher::try_send(
            event_dispatcher,
            cursor.create_change_event(),
            event.sender(),
            &mut commands,
        );
    }
}

impl_event_receiver! {
    impl EventReceiver<UiInputEvent> for UiTextCursor => text_cursor_on_input_system
}

pub fn update_text_cursor_layout_system(
    mut query: Query<(Ref<UiTextArea>, &mut UiTextCursor)>,
    text_query: Query<Ref<TextLayoutInfo>>,
    mut cursor_query: Query<(&mut Node, &mut BackgroundColor)>,
) {
    for (textarea, mut cursor) in query.iter_mut() {
        if textarea.is_changed() {
            let line_height = textarea.font_size * 1.2;
            if cursor.line_height != line_height {
                cursor.line_height = line_height;
            }
        }
        let Ok(text_layout) = text_query.get(textarea.text_entity) else {
            continue;
        };

        if cursor.is_changed() || text_layout.is_changed() {
            let gr_index = cursor.byte_index_to_glyph_index(&textarea, cursor.byte_index);
            cursor.position = cursor.get_cursor_position_of_glyph(gr_index, &text_layout);

            let Ok((mut cursor_node, mut background_color)) =
                cursor_query.get_mut(cursor.cursor_entity)
            else {
                continue;
            };

            cursor_node.left = Val::Px(cursor.position.x);
            cursor_node.top = Val::Px(cursor.position.y - cursor.line_height * 0.5);
            background_color.0 = cursor.corsor_color;
        }
    }
}

#[derive(Clone, Debug)]
pub enum UiTextCursorEvent {
    ChangePosition { position: Vec2, byte_index: usize },
    TextLayoutChanged {},
}
