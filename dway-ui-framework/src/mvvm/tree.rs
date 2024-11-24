use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use bevy::ecs::system::EntityCommands;

use super::{
    layout::{ItemLayout, ViewAreaLayout, ViewLayouter},
    ContainerViewFactory, ContainerViewModel, DataItem, DynEntityCommand, EntityWorldRef,
    IndexTrait, RangeModel, UpdateModel, ViewFactory,
};
use crate::prelude::*;

pub struct TreeRangeModel<NodeId> {
    pub items: BTreeSet<NodeId>,
}

impl<NodeId> Default for TreeRangeModel<NodeId> {
    fn default() -> Self {
        Self {
            items: Default::default(),
        }
    }
}

impl<NodeId: Ord> RangeModel<NodeId> for TreeRangeModel<NodeId> {
    fn in_range(&self, index: &NodeId) -> bool {
        self.items.contains(index)
    }

    fn upper_bound(&self) -> Option<&NodeId> {
        self.items.last()
    }

    fn lower_bound(&self) -> Option<&NodeId> {
        self.items.first()
    }
}

#[bevy_trait_query::queryable]
pub trait TreeViewModel<NodeId, Item: DataItem>:
    ContainerViewModel<NodeId, Item, UpdateModel = UpdateModel<NodeId, Item, TreeRangeModel<NodeId>>>
{
    fn get_root(&self, entity: EntityWorldRef) -> Vec<(NodeId, Item)>;
    fn get_root_by_index(&self, entity: EntityWorldRef) -> Option<(NodeId, Item)>;
    fn get_children(&self, node: &NodeId, entity: EntityWorldRef) -> Option<Vec<(NodeId, Item)>>;
    fn get_children_index(&self, node: &NodeId, entity: EntityWorldRef) -> Option<Vec<NodeId>>;
}

#[bevy_trait_query::queryable]
pub trait TreeItemViewFactory<NodeId, Item: DataItem>: ContainerViewFactory<NodeId, Item> {}

impl<
        Item: DataItem,
        T: ViewFactory<Item> + ContainerViewFactory<NodeId, Item>,
        NodeId: 'static,
    > TreeItemViewFactory<NodeId, Item> for T
{
}

pub struct TreeItemLayout<NodeId> {
    pub layout: ItemLayout,
    pub parent: Option<NodeId>,
    pub children: Option<Vec<NodeId>>,
    pub level: usize,
    pub sub_tree_size: Rect,
}

#[derive(Component)]
pub struct TreeViewLayout<NodeId: IndexTrait> {
    pub expand_by_default: bool,
    pub item_size: Vec2,
    pub items: BTreeMap<NodeId, TreeItemLayout<NodeId>>,
    pub view_area: ViewAreaLayout,
    pub used_area: Rect,
    pub dirty_area: Rect,
}

impl<NodeId: IndexTrait> TreeViewLayout<NodeId> {
    pub fn row_count(&self) -> usize {
        (self.view_area.rect.height() / self.item_size.y).ceil() as usize
    }

    pub fn has_space(&self) -> bool {
        self.used_area.max.y < self.view_area.rect.max.y
    }

    fn add<'l>(&'l mut self, entity: EntityCommands, index: NodeId) -> ItemLayout {
        todo!()
    }
}

impl<NodeId: IndexTrait> ViewLayouter<NodeId> for TreeViewLayout<NodeId> {
    fn contains(&self, index: NodeId) -> bool {
        todo!()
    }

    fn add<'l>(&'l mut self, entity: EntityCommands, index: NodeId) -> ItemLayout {
        todo!()
    }

    fn remove(&mut self, entity: EntityCommands, index: NodeId) {
        todo!()
    }

    fn get_item_layout(&self, index: NodeId) -> Option<&ItemLayout> {
        todo!()
    }

    fn get_item_layout_mut(&mut self, index: NodeId) -> Option<&mut ItemLayout> {
        todo!()
    }

    fn set_view_rect(&mut self, rect: Rect) {
        todo!()
    }

    fn truncate(&mut self, commands: Commands) -> Vec<NodeId> {
        todo!()
    }
}

#[bevy_trait_query::queryable]
pub trait TreeViewTrait<NodeId> {
    fn add(&mut self, commands: EntityCommands, item_index: NodeId, item_view_entity: Entity);
    fn remove(&mut self, commands: EntityCommands, item_index: NodeId);
    fn get_entity(&self, item_index: &NodeId) -> Option<Entity>;
    fn set_size(&mut self, commands: EntityCommands, size: Vec2);
}

fn tree_node_bind_data<NodeId: IndexTrait, Item: DataItem>(
    layout: &mut TreeViewLayout<NodeId>,
    id: &NodeId,
    model: &dyn TreeViewModel<NodeId, Item>,
    entity_ref: EntityWorldRef<'_>,
    mut entity_commands: EntityCommands<'_>,
) {
    let children = model.get_children(id, entity_ref.clone());
    let item_layout = layout.add(entity_commands.reborrow(), id.clone());
    if let Some(children) = &children {
        for (child_index, child) in children {
            if !layout.has_space() {
                break;
            }
            tree_node_bind_data(
                layout,
                &child_index,
                model,
                entity_ref.clone(),
                entity_commands.reborrow(),
            );
        }
    }
}

pub fn tree_bind_data<NodeId: IndexTrait, Item: DataItem>(
    mut set: ParamSet<(
        (
            &World,
            Query<(
                Entity,
                &TreeViewLayout<NodeId>,
                One<&dyn TreeViewModel<NodeId, Item>>,
            )>,
        ),
        Query<(
            Entity,
            &mut TreeViewLayout<NodeId>,
            One<&mut dyn TreeViewTrait<NodeId>>,
            One<&dyn TreeItemViewFactory<NodeId, Item>>,
        )>,
    )>,
    mut commands: Commands,
) {
    let (world, query) = set.p0();
    let mut update_list = vec![];
    for (container_entity, layout, model) in &query {
        let entity_ref = EntityWorldRef::new(world, container_entity);
        if model.update_from_world(entity_ref) {
            let changed = model.get_changed(entity_ref);
            update_list.push((
                container_entity,
                changed,
                // model.get_changed_in_range(entity_ref, layout.range.clone()), // TODO
            ));
        }
    }
    let mut query = set.p1();
    for (container_entity, update_model) in update_list {
        let Ok((container_entity, mut layout, mut view, item_factory)) =
            query.get_mut(container_entity)
        else {
            continue;
        };
        for (index, changed_item) in update_model.items {
            let item_layout = layout.add(commands.entity(container_entity), index.clone());
            let entity = if let Some(item_entity) = view.get_entity(&index) {
                item_entity
            } else {
                let entity = commands
                    .spawn(MiniNodeBundle {
                        style: Style {
                            top: Val::Px(item_layout.rect.min.y),
                            height: Val::Px(item_layout.rect.height()),
                            position_type: PositionType::Absolute,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .id();
                item_factory.create_item(&index, commands.entity(entity), changed_item);
                view.add(commands.entity(container_entity), index, entity);
                entity
            };
        }
        if let Some(range) = update_model.range {
            view.set_size(
                commands.entity(container_entity),
                Vec2::new(
                    layout.item_size.x,
                    layout.item_size.y * range.items.len() as f32,
                ),
            );
        }
        for removed_index in layout.truncate(commands.reborrow()) {
            view.remove(commands.entity(container_entity), removed_index);
        }
    }
}
