use std::ops::Range;

use bevy::{
    ecs::{
        component::{ComponentId, HookContext},
        world::DeferredWorld,
    },
    text::TextLayoutInfo,
    ui::{ContentSize, RelativeCursorPosition},
};
use bevy_prototype_lyon::{
    draw::{Fill, Stroke},
    entity::Shape,
    prelude::{ShapeBuilder, ShapeBuilderBase, *},
};
use bevy_svg::prelude::{FillOptions, StrokeOptions};

use super::{
    cursor::{UiTextCursor, UiTextCursorEvent},
    textarea::UiTextArea,
};
use crate::{
    impl_event_receiver,
    prelude::*,
    render::{mesh::UiMeshTransform, UiRenderOffset},
    widgets::shape::UiShape,
};

#[derive(Component, SmartDefault, Reflect)]
#[require(UiTextCursor)]
#[component(on_insert=on_insert_selection)]
#[component(on_replace=on_replace_selection)]
pub struct UiTextSelection {
    pub glyph_start: usize,
    pub glyph_end: usize,
    #[default(color!("#0000ff"))]
    pub color: Color,
    #[default(Entity::PLACEHOLDER)]
    path_entity: Entity,
}

impl UiTextSelection {
    pub fn get_range(&self) -> Option<Range<usize>> {
        if self.glyph_end == self.glyph_start {
            None
        } else {
            let glyph_start = usize::min(self.glyph_start, self.glyph_end);
            let glyph_end = usize::max(self.glyph_start, self.glyph_end);
            Some(glyph_start..glyph_end)
        }
    }

    pub fn build_path(
        &self,
        cursor: &UiTextCursor,
        text_layout: &TextLayoutInfo,
        size: Vec2,
        line_width: f32,
    ) -> ShapePath {
        let Some(Range {
            start: glyph_start,
            end: glyph_end,
        }) = self.get_range()
        else {
            return ShapePath::default();
        };

        let start_position = cursor.get_cursor_position_of_glyph(Some(glyph_start), text_layout);
        let end_position = cursor.get_cursor_position_of_glyph(Some(glyph_end), text_layout);
        let line_height = cursor.line_height;
        let multi_line = (start_position.y - end_position.y).abs() > 0.5 * line_height;
        let two_line = (start_position.y - end_position.y).abs() < 1.5 * line_height;

        let first_line_top = start_position.y - 0.5 * line_height + 0.5 * line_width;
        let first_line_bottom = start_position.y + 0.5 * line_height - 0.5 * line_width;
        let first_line_left = start_position.x + 0.5 * line_width;
        let first_line_right = size.x - 0.5 * line_width;
        let last_line_top = end_position.y - 0.5 * line_height + 0.5 * line_width;
        let last_line_bottom = end_position.y + 0.5 * line_height - 0.5 * line_width;
        let last_line_left = 0.5 * line_width;
        let last_line_right = end_position.x - 0.5 * line_width;

        if multi_line && two_line {
            ShapePath::new()
                .move_to(Vec2::new(first_line_left, first_line_top))
                .line_to(Vec2::new(first_line_right, first_line_top))
                .line_to(Vec2::new(first_line_right, first_line_bottom))
                .line_to(Vec2::new(first_line_left, first_line_bottom))
                .close()
                .move_to(Vec2::new(last_line_left, last_line_top))
                .line_to(Vec2::new(last_line_right, last_line_top))
                .line_to(Vec2::new(last_line_right, last_line_bottom))
                .line_to(Vec2::new(last_line_left, last_line_bottom))
                .close()
        } else if multi_line {
            let first_line_bottom = start_position.y + 0.5 * line_height + 0.5 * line_width;
            let last_line_top = end_position.y - 0.5 * line_height - 0.5 * line_width;
            ShapePath::new()
                .move_to(Vec2::new(first_line_left, first_line_top))
                .line_to(Vec2::new(first_line_right, first_line_top))
                .line_to(Vec2::new(first_line_right, last_line_top))
                .line_to(Vec2::new(last_line_right, last_line_top))
                .line_to(Vec2::new(last_line_right, last_line_bottom))
                .line_to(Vec2::new(last_line_left, last_line_bottom))
                .line_to(Vec2::new(last_line_left, first_line_bottom))
                .line_to(Vec2::new(first_line_left, first_line_bottom))
                .close()
        } else {
            ShapePath::new()
                .move_to(Vec2::new(first_line_left, first_line_top))
                .line_to(Vec2::new(last_line_right, first_line_top))
                .line_to(Vec2::new(last_line_right, first_line_bottom))
                .line_to(Vec2::new(first_line_left, first_line_bottom))
                .close()
        }
    }

    pub fn path_entity(&self) -> Entity {
        self.path_entity
    }
}

pub fn on_insert_selection(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let selection = world.get_mut::<UiTextSelection>(entity).unwrap();
    let color = selection.color;

    if selection.path_entity == Entity::PLACEHOLDER {
        let path_entity = world
            .commands()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
                (
                    UiShape::default(),
                    UiMeshTransform::new_ui_transform(),
                    ShapeBuilder::with(&ShapePath::default())
                        .fill(Fill {
                            options: FillOptions::default(),
                            color,
                        })
                        .stroke(Stroke {
                            options: StrokeOptions::default()
                                .with_line_join(bevy_svg::prelude::LineJoin::Round)
                                .with_end_cap(bevy_svg::prelude::LineCap::Round)
                                .with_start_cap(bevy_svg::prelude::LineCap::Round)
                                .with_line_width(8.0),
                            color,
                        })
                        .build(),
                ),
                ZIndex(crate::widgets::zindex::TEXT_SELECTION),
                UiRenderOffset(crate::widgets::zoffset::TEXT_SELECTION),
            ))
            .set_parent(entity)
            .id();

        let mut selection = world.get_mut::<UiTextSelection>(entity).unwrap();
        selection.path_entity = path_entity;
    }
}

pub fn on_replace_selection(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let selection = world.get_mut::<UiTextSelection>(entity).unwrap();
    let path_entity = selection.path_entity;
    world.commands().queue(move |world: &mut World| {
        if let Ok(entity_mut) = world.get_entity_mut(path_entity) {
            entity_mut.despawn_recursive();
        }
    });
}

pub fn on_ui_input_event(
    event: UiEvent<UiTextCursorEvent>,
    mut self_query: Query<(&mut UiTextSelection, &Interaction, &UiInput)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    let Ok((mut selection, interaction, ui_input)) = self_query.get_mut(event.sender()) else {
        return;
    };

    match &*event {
        UiTextCursorEvent::ChangePosition { byte_index, .. } => {
            let byte_index = *byte_index;
            if *interaction == Interaction::Pressed {
                let shift_pressed = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
                if ui_input.pressed() && shift_pressed {
                    if (byte_index < (selection.glyph_start + selection.glyph_end) / 2)
                        ^ (selection.glyph_start < selection.glyph_end)
                    {
                        selection.glyph_end = byte_index;
                    } else {
                        selection.glyph_start = byte_index;
                    }
                } else if ui_input.just_pressed() {
                    selection.glyph_start = byte_index;
                    selection.glyph_end = byte_index;
                } else {
                    selection.glyph_end = byte_index;
                }
            }
        }
        UiTextCursorEvent::TextLayoutChanged {} => {
            selection.set_changed();
        }
    }
}

impl_event_receiver! {
    impl EventReceiver<UiTextCursorEvent> for UiTextSelection => on_ui_input_event
}

pub fn update_ui_text_selection_system(
    mut ui_query: Query<
        (&UiTextCursor, &UiTextSelection, &UiTextArea, &ComputedNode),
        Or<(Changed<UiTextSelection>, Changed<ComputedNode>)>,
    >,
    text_query: Query<Ref<TextLayoutInfo>>,
    mut path_query: Query<(&mut Shape, &mut UiMeshTransform)>,
) {
    for (cursor, selection, textarea, node) in ui_query.iter_mut() {
        let Ok((mut shape, mut mesh_transform)) = path_query.get_mut(selection.path_entity) else {
            warn!(
                "can not query Path and Fill component from {:?}",
                selection.path_entity
            );
            continue;
        };
        let Ok(text_layout) = text_query.get(textarea.text_entity) else {
            continue;
        };

        let size = node.size();
        let path = selection.build_path(
            cursor,
            &text_layout,
            size,
            shape
                .stroke
                .map(|s| s.options.line_width)
                .unwrap_or_default(),
        );

        let color = selection.color;
        *shape = ShapeBuilder::with(&path)
            .fill(shape.fill.unwrap_or_default())
            .stroke(Stroke {
                options: shape.stroke.map(|s| s.options).unwrap_or_default(),
                color,
            })
            .build();

        mesh_transform.translation.x = -0.5 * size.x;
        mesh_transform.translation.y = -0.5 * size.y;
    }
}
