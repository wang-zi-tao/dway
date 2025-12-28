use std::{marker::PhantomData, ops::Range};

use super::{
    list::{ListRangeModel, ListViewModel},
    ContainerViewModel, DataItem, EntityCommands, EntityWorldRef,
    UpdateModel, ViewModel,
};
use crate::prelude::*;

#[derive(Component)]
pub struct SimpleItemViewModel<Item: DataItem>(pub Item);

impl<Item: DataItem> SimpleItemViewModel<Item> {
    fn update(&self, _entity: EntityRef, _item: Item) {
    }
}

impl<Item: DataItem + Clone> ViewModel<Item> for SimpleItemViewModel<Item> {
    fn is_changed(&self, entity: EntityWorldRef) -> bool {
        entity.get().get_ref::<Self>().unwrap().is_changed()
    }

    fn get(&self, _entity: EntityWorldRef) -> Item {
        self.0.clone()
    }

    fn set(&self, mut commands: EntityCommands, item: Item) {
        commands.queue(move |mut entity: EntityWorldMut| {
            entity.get_mut::<Self>().unwrap().0 = item;
        });
    }
}

#[derive(Component)]
pub struct SimpleListViewModel<Item: DataItem>(pub Vec<Item>);

impl<Item: DataItem + Clone> ListViewModel<Item> for SimpleListViewModel<Item> {
    fn len(&self, _world: EntityWorldRef<'_>) -> usize {
        self.0.len()
    }

    fn get_changed_in_range(
        &self,
        _entity: EntityWorldRef,
        range: Range<usize>,
    ) -> Vec<(usize, Item)> {
        self.0
            .iter()
            .cloned()
            .enumerate()
            .skip(range.start)
            .take(range.len())
            .collect()
    }
}

impl<Item: DataItem + Clone> ContainerViewModel<usize, Item> for SimpleListViewModel<Item> {
    type UpdateModel = UpdateModel<usize, Item, ListRangeModel>;

    fn update_from_world(&self, entity: EntityWorldRef) -> bool {
        entity.get().get_ref::<Self>().unwrap().is_changed()
    }

    fn get(&self, key: &usize, _entity: EntityWorldRef) -> Option<Item> {
        self.0.get(*key).cloned()
    }

    fn get_changed(&self, entity: EntityWorldRef) -> Self::UpdateModel {
        let changed = self.update_from_world(entity);
        UpdateModel {
            items: if changed {
                self.0.iter().cloned().enumerate().collect()
            } else {
                vec![]
            },
            range: changed.then_some(ListRangeModel(0..self.0.len())),
        }
    }

    fn set(&self, key: &usize, mut commands: EntityCommands, item: Item) {
        let key = *key;
        commands.queue(move |mut c: EntityWorldMut| {
            if let Some(p) = c.get_mut::<Self>().unwrap().0.get_mut(key) {
                *p = item;
            };
        });
    }

    fn set_batch(
        &self,
        mut commands: EntityCommands,
        items: &mut dyn Iterator<Item = (usize, Item)>,
    ) {
        let items = Vec::from_iter(items);
        commands.queue(|mut c: EntityWorldMut| {
            let mut this = c.get_mut::<Self>().unwrap();
            for (index, item) in items {
                if let Some(p) = this.0.get_mut(index) {
                    *p = item;
                }
            }
        });
    }
}

#[derive(Default)]
pub struct ViewModelPlugin<Item: DataItem>(PhantomData<Item>);
impl<Item: DataItem + Clone> Plugin for ViewModelPlugin<Item> {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn ViewModel<Item>, SimpleItemViewModel<Item>>();
        app.register_component_as::<dyn ListViewModel<Item>, SimpleListViewModel<Item>>();
    }
}
