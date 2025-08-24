pub mod container;
pub mod graph;
pub mod layout;
pub mod list;
pub mod selection;
pub mod table;
pub mod tree;
pub mod view;
pub mod viewmodel;

use std::{any::Any, hash::Hash, marker::PhantomData};

use bevy::ecs::system::EntityCommands;
use list::{ListItemViewFactory, ListViewLayout, ListViewTrait};
use view::{list::ListView, TextViewFactory};

use crate::{prelude::*, UiFrameworkSystems};

pub type DynEntityCommand = Box<dyn FnOnce(EntityWorldMut) + Send + 'static>;

pub trait DataItem: Send + Sync + Any {}
impl<T: Send + Sync + Any> DataItem for T {
}

pub trait IndexTrait: Send + Sync + Ord + Eq + Hash + 'static + Clone {}

bitflags::bitflags! {
    pub struct ViewItemState: u8 {
        const SELECTED = 1;
        const FOCUSED = 1 << 1;
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

pub trait RangeModel<Index>: Default {
    fn in_range(&self, index: &Index) -> bool;
    fn upper_bound(&self) -> Option<&Index>;
    fn lower_bound(&self) -> Option<&Index>;
}

impl RangeModel<()> for () {
    fn in_range(&self, _: &()) -> bool {
        true
    }

    fn upper_bound(&self) -> Option<&()> {
        None
    }

    fn lower_bound(&self) -> Option<&()> {
        None
    }
}

pub trait UpdateModelTrait<Index, Item: DataItem> {
    type Range: RangeModel<Index>;
    fn get_changed(&self) -> &[(Index, Item)];
    fn range_changed(&self) -> Option<&Self::Range>;
}

pub struct UpdateModel<Index, Item: DataItem, Range> {
    pub items: Vec<(Index, Item)>,
    pub range: Option<Range>,
}

impl<Index, Item: DataItem, Range: RangeModel<Index>> UpdateModelTrait<Index, Item>
    for UpdateModel<Index, Item, Range>
{
    type Range = Range;

    fn get_changed(&self) -> &[(Index, Item)] {
        &self.items
    }

    fn range_changed(&self) -> Option<&Self::Range> {
        self.range.as_ref()
    }
}

pub trait ContainerViewModel<Index, Item: DataItem> {
    type UpdateModel: UpdateModelTrait<Index, Item>;
    fn update_from_world(&self, entity: EntityWorldRef) -> bool;
    fn get(&self, key: &Index, entity: EntityWorldRef) -> Option<Item>;
    fn get_changed(&self, entity: EntityWorldRef) -> Self::UpdateModel;
    fn set(&self, key: &Index, commands: EntityCommands, item: Item);
    fn set_batch(&self, commands: EntityCommands, items: &mut dyn Iterator<Item = (Index, Item)>);
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

pub trait ContainerViewFactory<Key, Item: DataItem> {
    fn create_item(&self, key: &Key, commands: EntityCommands, item: Item);
}

#[bevy_trait_query::queryable]
pub trait ViewFactory<Item: DataItem> {
    fn create(&self, commands: EntityCommands, item: Item);
}

impl<Item: DataItem, Index, T: ViewFactory<Item>> ContainerViewFactory<Index, Item> for T {
    fn create_item(&self, _key: &Index, commands: EntityCommands, item: Item) {
        self.create(commands, item)
    }
}

#[derive(Default, PartialEq, Eq)]
pub struct MvvmPlugin;
impl Plugin for MvvmPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn ListViewTrait, ListView>()
            .register_type::<ListView>()
            .add_systems(
                PostUpdate,
                ListView::update_layout.in_set(UiFrameworkSystems::UpdateViewLayout),
            )
            .register_type::<ListViewLayout>()
            .add_plugins(ViewFactoryPlugin::<String, TextViewFactory>::default());
    }
}

#[derive(Default, PartialEq, Eq)]
pub struct ViewFactoryPlugin<Item: DataItem, Impl: Component>(PhantomData<(Item, Impl)>);

impl<Item: DataItem, Impl: Component + ViewFactory<Item>> Plugin for ViewFactoryPlugin<Item, Impl> {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn ViewFactory<Item>, Impl>();
        app.register_component_as::<dyn ListItemViewFactory<Item>, Impl>();
    }
}
