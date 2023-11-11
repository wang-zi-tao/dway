use crate::prelude::*;

#[derive(Component)]
pub struct Background(pub Entity);

pub fn update_background_sprite(query: Query<&Background>) {}
