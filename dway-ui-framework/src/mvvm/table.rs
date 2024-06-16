use super::{ContainerViewModel, DataItem, ViewFactory};
use crate::prelude::*;
use bevy::ecs::system::EntityCommands;
use std::{marker::PhantomData, ops::Range};

#[bevy_trait_query::queryable]
pub trait TableItemViewFactory<Item: DataItem>: ContainerViewModel<[usize; 2], Item> {
    fn create_raw(&self, _index: usize, _commands: EntityCommands) {}
}
impl<Item: DataItem, T: ViewFactory<Item> + ContainerViewModel<[usize; 2], Item>>
    TableItemViewFactory<Item> for T
{
}

#[bevy_trait_query::queryable]
pub trait TableViewModel<Item: DataItem> {
    fn len(&self) -> [usize; 2];
    fn get(&self, index: [usize; 2]) -> Item;
    fn update(&self, index: [usize; 2], item: Item) -> bool;
    fn update_all(
        &self,
        range1: Range<usize>,
        range2: Range<usize>,
        items: &dyn Iterator<Item = Item>,
    ) -> bool;
}

#[derive(Component)]
pub struct Table<Item: DataItem> {
    pub phantom: PhantomData<Item>,
}

impl<Item: DataItem> Default for Table<Item> {
    fn default() -> Self {
        Self {
            phantom: Default::default(),
        }
    }
}

dway_widget! {
Table<Item: DataItem>=>
@use_state(phantim: PhantomData<Item>)
<MiniNodeBundle>
</MiniNodeBundle>
}
