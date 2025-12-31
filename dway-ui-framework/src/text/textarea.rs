use bevy::ecs::world::DeferredWorld;

use crate::prelude::*;

#[derive(Component, Reflect)]
#[require(Node)]
#[component(on_insert=on_insert_text_area)]
pub struct UiTextArea {
    pub(crate) data: String,
    pub(crate) text_entity: Entity,
    pub color: TextColor,
    pub font: TextFont,
}

impl UiTextArea {
    pub fn new(data: impl ToString, font_size: f32) -> Self {
        Self {
            data: data.to_string(),
            text_entity: Entity::PLACEHOLDER,
            color: TextColor(Color::BLACK),
            font: TextFont {
                font_size,
                ..Default::default()
            },
        }
    }

    pub fn with_font(mut self, font: TextFont) -> Self {
        self.font = font;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = TextColor(color);
        self
    }
}

pub fn on_insert_text_area(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let textarea = world.get_mut::<UiTextArea>(entity).unwrap();
    let text_color = textarea.color;
    let font = textarea.font.clone();
    let data = textarea.data.clone();

    if textarea.text_entity == Entity::PLACEHOLDER {
        let text_entity = world
            .commands()
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                Text(data),
                text_color,
                font,
                ChildOf(entity),
            ))
            .id();
        let mut textarea = world.get_mut::<UiTextArea>(entity).unwrap();
        textarea.text_entity = text_entity;
    }
}

pub fn update_textarea(
    query: Query<&UiTextArea, Changed<UiTextArea>>,
    mut text_query: Query<(&mut Text, &mut TextColor, &mut TextFont)>,
) {
    for textarea in query.iter() {
        let Ok((mut text, mut color, mut font)) = text_query.get_mut(textarea.text_entity) else {
            return;
        };

        text.set_if_neq(Text(textarea.data.clone()));
        color.set_if_neq(textarea.color);
        font.set_if_neq(textarea.font.clone());
    }
}
