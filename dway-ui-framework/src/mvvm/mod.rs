pub mod container;
pub mod graph;
pub mod list;
pub mod table;
pub mod tree;
pub mod view;
pub mod viewmode;

use crate::prelude::*;
use bevy::ecs::system::EntityCommands;
use std::any::Any;

pub type DynEntityCommand = Box<dyn FnOnce(EntityWorldMut) + Send + 'static>;

pub trait DataItem: Send + Sync + Any {}
impl<T: Send + Sync + Any> DataItem for T {}

bitflags::bitflags! {
    pub struct ViewItemState: u8 {
        const SELECTED = 1;
    }
}

#[derive(Component)]
pub struct ViewItem<Item: DataItem> {
    pub item: Item,
    pub state: ViewItemState,
}

impl<Item: DataItem> ViewItem<Item> {
    pub fn new(item: Item) -> Self {
        Self {
            item,
            state: ViewItemState::empty(),
        }
    }

    pub fn set_item(&mut self, item: Item) {
        self.item = item;
    }
}

#[derive(Clone, Copy)]
pub struct EntityWorldRef<'a> {
    pub world: &'a World,
    pub entity: Entity,
}

impl<'a> EntityWorldRef<'a> {
    pub fn new(world: &'a World, entity: Entity) -> Self {
        Self { world, entity }
    }
    pub fn get(&self) -> EntityRef {
        self.world.entity(self.entity)
    }
}

pub trait ContainerViewModel<Key, Item: DataItem> {
    fn update_from_world(&self, entity: EntityWorldRef) -> bool;
    fn get(&self, key: &Key, entity: EntityWorldRef) -> Option<Item>;
    fn get_changed(&self, entity: EntityWorldRef) -> Box<dyn Iterator<Item = (Key, Item)>>;
    fn set(&self, key: &Key, commands: EntityCommands, item: Item);
    fn set_batch(&self, commands: EntityCommands, items: &mut dyn Iterator<Item = (Key, Item)>);
}

#[bevy_trait_query::queryable]
pub trait ViewModel<Item: DataItem> {
    fn is_changed(&self, entity: EntityWorldRef) -> bool;
    fn update_from_world(&self, entity: EntityWorldRef) -> bool {
        self.is_changed(entity)
    }
    fn get(&self, entity: EntityWorldRef) -> Item;
    fn set(&self, info: EntityCommands, item: Item);
}

impl<Item: DataItem, T: ViewModel<Item>> ContainerViewModel<(), Item> for T {
    fn update_from_world(&self, entity: EntityWorldRef) -> bool {
        self.update_from_world(entity)
    }

    fn get(&self, _key: &(), entity: EntityWorldRef) -> Option<Item> {
        Some(self.get(entity))
    }

    fn get_changed(&self, entity: EntityWorldRef) -> Box<dyn Iterator<Item = ((), Item)>> {
        Box::new(
            self.is_changed(entity)
                .then(|| ((), self.get(entity)))
                .into_iter(),
        )
    }

    fn set(&self, _key: &(), commands: EntityCommands, item: Item) {
        self.set(commands, item)
    }

    fn set_batch(&self, commands: EntityCommands, items: &mut dyn Iterator<Item = ((), Item)>) {
        if let Some(((), item)) = items.last() {
            self.set(commands, item);
        }
    }
}

pub trait ContainerViewFactory<Key, Item: DataItem> {
    fn create(&self, key: &Key, commands: EntityCommands, item: Item);
}

#[bevy_trait_query::queryable]
pub trait ViewFactory<Item: DataItem> {
    fn create(&self, commands: EntityCommands, item: Item);
}

impl<Item: DataItem, T: ViewFactory<Item>> ContainerViewFactory<(), Item> for T {
    fn create(&self, _key: &(), commands: EntityCommands, item: Item) {
        self.create(commands, item)
    }
}

pub struct MvvmPlugin;
impl Plugin for MvvmPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn ViewFactory<String>, view::TextViewFactory>();
    }
}
