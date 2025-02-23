pub mod list;
use bevy::ecs::system::EntityCommands;

use super::ViewFactory;
use crate::prelude::*;

#[derive(Component, Default)]
pub struct TextViewFactory {
    pub font: TextFont,
    pub color: TextColor,
}

impl TextViewFactory {
    pub fn new(font: TextFont, color: TextColor) -> Self {
        Self { font, color }
    }
}
impl ViewFactory<String> for TextViewFactory {
    fn create(&self, mut info: EntityCommands, item: String) {
        let font = self.font.clone();
        let color = self.color;
        info.queue(move |mut entity: EntityWorldMut| {
            if let Some(mut text) = entity.get_mut::<Text>() {
                *text = Text::new(item);
            } else {
                entity.insert((Text::new(item), font, color));
            }
        });
    }
}
