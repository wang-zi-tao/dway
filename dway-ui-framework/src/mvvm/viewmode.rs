use std::marker::PhantomData;

use super::{
    ContainerViewModel, DataItem, DynEntityCommand, EntityCommands, EntityWorldRef, ViewModel,
};
use crate::prelude::*;

#[derive(Component)]
pub struct SimpleItemViewModel<Item: DataItem>(pub Item);

impl<Item: DataItem> SimpleItemViewModel<Item> {
    fn update(&self, entity: EntityRef, item: Item) {}
}

impl<Item: DataItem + Clone> ViewModel<Item> for SimpleItemViewModel<Item> {
    fn is_changed(&self, entity: EntityWorldRef) -> bool {
        entity.get().get_ref::<Self>().unwrap().is_changed()
    }

    fn get(&self, _entity: EntityWorldRef) -> Item {
        self.0.clone()
    }

    fn set(&self, mut commands: EntityCommands, item: Item) {
        commands.add(move |mut entity: EntityWorldMut| {
            entity.get_mut::<Self>().unwrap().0 = item;
        });
    }
}

#[derive(Component)]
pub struct VecViewModel<Item: DataItem>(pub Vec<Item>);

impl<Item: DataItem + Clone> ContainerViewModel<usize, Item> for VecViewModel<Item> {
    fn update_from_world(&self, entity: EntityWorldRef) -> bool {
        entity.get().get_ref::<Self>().unwrap().is_changed()
    }

    fn get(&self, key: &usize, entity: EntityWorldRef) -> Option<Item> {
        self.0.get(*key).cloned()
    }

    fn get_changed(&self, entity: EntityWorldRef) -> Box<dyn Iterator<Item = (usize, Item)>> {
        Box::new(
            entity
                .get()
                .get_ref::<Self>()
                .unwrap()
                .is_changed()
                .then(|| self.0.clone())
                .unwrap_or_default()
                .into_iter()
                .enumerate(),
        )
    }

    fn set(&self, key: &usize, mut commands: EntityCommands, item: Item) {
        let key = *key;
        commands.add(move |mut c: EntityWorldMut| {
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
        commands.add(|mut c: EntityWorldMut| {
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
    }
}
