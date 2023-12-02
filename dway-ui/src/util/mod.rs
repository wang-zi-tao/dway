use dway_server::util::rect::IRect;

use crate::prelude::*;

#[macro_use]
pub mod create_widget;

pub fn irect_to_style(rect: IRect) -> Style {
    Style {
        position_type: PositionType::Absolute,
        left: Val::Px(rect.x() as f32),
        top: Val::Px(rect.y() as f32),
        width: Val::Px(rect.width() as f32),
        height: Val::Px(rect.height() as f32),
        ..Default::default()
    }
}
