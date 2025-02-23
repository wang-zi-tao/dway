use std::marker::PhantomData;

use super::{DataItem, EntityWorldRef, ViewFactory, ViewModel};
use crate::prelude::*;

#[derive(Component, Default)]
pub struct ItemCell<Item: DataItem> {
    phantom: PhantomData<Item>,
}

impl<Item: DataItem> ItemCell<Item> {
    pub fn update_system(
        world: &World,
        mut query: Query<
            (
                Entity,
                One<&dyn ViewModel<Item>>,
                One<&dyn ViewFactory<Item>>,
                Option<&CalculatedClip>,
            ),
            With<Self>,
        >,
        mut commands: Commands,
    ) {
        for (entity, model, view, clip) in &mut query {
            if clip.map(|c| c.clip.is_empty()).unwrap_or(false) {
                continue;
            }
            let entity_world_ref = EntityWorldRef::new(world, entity);
            if model.update_from_world(entity_world_ref) {
                let item = model.get(entity_world_ref);
                view.create(commands.entity(entity), item);
            }
        }
    }
}

#[derive(Default)]
pub struct ItemCellPlugin<Item: DataItem>(PhantomData<Item>);
impl<Item: DataItem> Plugin for ItemCellPlugin<Item> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            ItemCell::<Item>::update_system.in_set(UiFrameworkSystems::UpdateMVVM),
        );
    }
}
