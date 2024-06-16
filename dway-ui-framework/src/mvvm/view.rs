use bevy::ecs::system::EntityCommands;

use super::ViewFactory;
use crate::{prelude::*, widgets::text::UiTextExt};

#[derive(Component, Default)]
pub struct TextViewFactory {
    pub style: TextStyle,
}

impl TextViewFactory {
    pub fn new(style: TextStyle) -> Self {
        Self { style }
    }
}
impl ViewFactory<String> for TextViewFactory {
    fn create(&self, mut info: EntityCommands, item: String) {
        let style = self.style.clone();
        info.add(move |mut entity: EntityWorldMut| {
            if let Some(mut text) = entity.get_mut::<Text>() {
                *text = Text::from_section(item, style);
            } else {
                entity.insert(UiTextExt {
                    text: Text::from_section(item, style),
                    ..Default::default()
                });
            }
        });
    }
}
