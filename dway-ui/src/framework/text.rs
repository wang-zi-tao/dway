use bevy::{
    text::TextLayoutInfo,
    ui::{widget::TextFlags, ContentSize},
};

use crate::prelude::*;

#[derive(Bundle, Default)]
pub struct UiTextBundle {
    pub node: MiniNodeBundle,
    pub addon: UiTextAddonBundle,
}

impl UiTextBundle {
    pub fn new(string: &str, size: usize, theme: &Theme) -> Self {
        Self {
            node: default(),
            addon: UiTextAddonBundle::new(string, size, theme),
        }
    }
}

#[derive(Bundle, Default)]
pub struct UiTextAddonBundle {
    pub text: Text,
    pub text_layout_info: TextLayoutInfo,
    pub text_flags: TextFlags,
    pub calculated_size: ContentSize,
}

impl UiTextAddonBundle {
    pub fn new(string: &str, size: usize, theme: &Theme) -> Self {
        Self {
            text: Text::from_section(
                string,
                TextStyle {
                    font: theme.default_font(),
                    font_size: size as f32,
                    color: theme.color("foreground"),
                },
            ),
            ..default()
        }
    }
}
