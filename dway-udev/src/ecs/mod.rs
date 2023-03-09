use bevy::prelude::*;
use smithay::utils::{Physical, Rectangle};

#[derive(Component)]
pub struct PhysicalRect(pub Rectangle<i32, Physical>);
impl PhysicalRect {
    pub fn width(&self) -> u32 {
        self.0.size.w as u32
    }

    pub(crate) fn height(&self) -> u32 {
        self.0.size.h as u32
    }

    pub(crate) fn size_vec2(&self) -> Vec2 {
        Vec2::new(self.0.size.w as f32, self.0.size.h as f32)
    }
}
