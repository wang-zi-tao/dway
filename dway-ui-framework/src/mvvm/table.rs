use std::{marker::PhantomData, ops::Range};

use bevy::ecs::system::EntityCommands;

use super::{ContainerViewModel, DataItem, RangeModel};
use crate::prelude::*;

pub struct TableRangeModel {
    pub min: [usize; 2],
    pub max: [usize; 2],
}

impl Default for TableRangeModel {
    fn default() -> Self {
        Self {
            min: [0; 2],
            max: [usize::MAX; 2],
        }
    }
}

impl RangeModel<[usize; 2]> for TableRangeModel {
    fn in_range(&self, &index: &[usize; 2]) -> bool {
        index >= self.min && index <= self.max
    }

    fn upper_bound(&self) -> Option<&[usize; 2]> {
        Some(&self.max)
    }

    fn lower_bound(&self) -> Option<&[usize; 2]> {
        Some(&self.min)
    }
}

pub struct TableUpdateModel<Item> {
    pub row: Option<usize>,
    pub col: Option<usize>,
    pub updated_items: Vec<([usize; 2], Item)>,
}

#[bevy_trait_query::queryable]
pub trait TableItemViewFactory<Item: DataItem>:
    ContainerViewModel<[usize; 2], Item, UpdateModel = TableUpdateModel<Item>>
{
    fn create_raw(&self, _index: usize, _commands: EntityCommands) {
    }
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
<Node/>
}
