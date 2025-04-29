use bevy::ecs::{component::ComponentId, world::DeferredWorld};

use crate::prelude::*;

#[derive(Component, Reflect)]
#[require(Node)]
#[component(on_insert=on_insert_text_area)]
pub struct UiTextArea {
    pub data: String,
    pub text_entity: Entity,
    pub font_size: f32,
    pub color: TextColor,
    pub font: TextFont,
}

impl UiTextArea {
    pub fn new(data: impl ToString, font_size: f32) -> Self {
        Self {
            data: data.to_string(),
            text_entity: Entity::PLACEHOLDER,
            font_size,
            color: TextColor(Color::BLACK),
            font: Default::default(),
        }
    }
}

pub fn on_insert_text_area(mut world: DeferredWorld, entity: Entity, _: ComponentId) {
    let textarea = world.get_mut::<UiTextArea>(entity).unwrap();
    let text_color = textarea.color;
    let font_size = textarea.font_size;
    let data = textarea.data.clone();

    if textarea.text_entity == Entity::PLACEHOLDER {
        let text_entity = world
            .commands()
            .spawn((
                Node{
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                Text(data),
                text_color,
                TextFont {
                    font_size,
                    ..Default::default()
                },
            ))
            .set_parent(entity)
            .id();
        let mut textarea = world.get_mut::<UiTextArea>(entity).unwrap();
        textarea.text_entity = text_entity;
    }
}

pub fn update_textarea(
    query: Query<&UiTextArea, Changed<UiTextArea>>,
    mut text_query: Query<&mut Text>,
) {
    for textarea in query.iter() {
        let Ok(mut text) = text_query.get_mut(textarea.text_entity) else {
            return;
        };
        text.0 = textarea.data.clone();
    }
}
