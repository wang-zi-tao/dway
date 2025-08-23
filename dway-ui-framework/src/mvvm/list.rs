use std::{collections::BTreeMap, marker::PhantomData, ops::Range};

use bevy::ecs::system::EntityCommands;
use derive_builder::Builder;
use derive_more::From;

use super::{
    layout::{ItemLayout, ViewAreaLayout, ViewLayouter},
    ContainerViewFactory, ContainerViewModel, DataItem, EntityWorldRef,
    RangeModel, UpdateModel, UpdateModelTrait,
};
use crate::prelude::*;

#[derive(Deref, DerefMut, From)]
pub struct ListRangeModel(pub Range<usize>);

impl Default for ListRangeModel {
    fn default() -> Self {
        Self(Range {
            start: 0,
            end: usize::MAX,
        })
    }
}

impl RangeModel<usize> for ListRangeModel {
    fn in_range(&self, index: &usize) -> bool {
        self.0.contains(index)
    }

    fn upper_bound(&self) -> Option<&usize> {
        Some(&self.0.end)
    }

    fn lower_bound(&self) -> Option<&usize> {
        Some(&self.0.start)
    }
}

#[bevy_trait_query::queryable]
pub trait ListViewModel<Item: DataItem>:
    ContainerViewModel<usize, Item, UpdateModel = UpdateModel<usize, Item, ListRangeModel>>
{
    fn len(&self, entity: EntityWorldRef) -> usize;
    fn get_changed_in_range(
        &self,
        entity: EntityWorldRef,
        range: Range<usize>,
    ) -> Vec<(usize, Item)>;
}

#[bevy_trait_query::queryable]
pub trait ListItemViewFactory<Item: DataItem>: ContainerViewFactory<usize, Item> {}

impl<Item: DataItem, T: ContainerViewFactory<usize, Item> + 'static> ListItemViewFactory<Item>
    for T
{
}

#[bevy_trait_query::queryable]
pub trait ListViewTrait {
    fn add(&mut self, commands: EntityCommands, item_index: usize, item_view_entity: Entity);
    fn remove(&mut self, commands: EntityCommands, item_index: usize);
    fn get_entity(&self, item_index: usize) -> Option<Entity>;
    fn set_size(&mut self, commands: EntityCommands, size: Vec2);
}

#[derive(Component, Default, Builder, Reflect, Clone)]
pub struct ListViewLayout {
    pub item_size: Vec2,
    pub items: BTreeMap<usize, ItemLayout>,
    pub view_area: ViewAreaLayout,
    pub range: Range<usize>,
}

impl ViewLayouter<usize> for ListViewLayout {
    fn add(&mut self, entity: EntityCommands, index: usize) -> ItemLayout {
        if let Some(layout) = self.items.get(&index) {
            return layout.clone();
        }
        let lower = self
            .items
            .lower_bound(std::ops::Bound::Included(&index))
            .peek_prev();
        let upper = self
            .items
            .upper_bound(std::ops::Bound::Included(&index))
            .peek_next();

        let position = match (lower, upper) {
            (None, None) => index as f32 * self.item_size.y,
            (None, Some(upper)) => upper.1.rect.min.y - (upper.0 - index) as f32 * self.item_size.y,
            (Some(lower), None) => {
                lower.1.rect.max.y + (index - lower.0 - 1) as f32 * self.item_size.y
            }
            (Some(lower), Some(upper)) => {
                if upper.0 - index >= index - lower.0 {
                    upper.1.rect.min.y - (upper.0 - index) as f32 * self.item_size.y
                } else {
                    lower.1.rect.max.y + (index - lower.0 - 1) as f32 * self.item_size.y
                }
            }
        };

        self.items.insert(
            index,
            ItemLayout {
                rect: Rect::new(0.0, position, self.item_size.x, position + self.item_size.y),
            },
        );
        self.items[&index].clone()
    }

    fn remove(&mut self, mut entiy: EntityCommands, index: usize) {
        self.items.remove(&index);
        entiy.despawn_recursive();
    }

    fn get_item_layout(&self, index: usize) -> Option<&ItemLayout> {
        self.items.get(&index)
    }

    fn get_item_layout_mut(&mut self, index: usize) -> Option<&mut ItemLayout> {
        self.items.get_mut(&index)
    }

    fn set_view_rect(&mut self, rect: Rect) {
        self.view_area.rect = rect;
    }

    fn truncate(&mut self, commands: Commands) -> Vec<usize> {
        let mut removed_items = vec![];
        self.items.retain(|k, v| {
            let r = v.rect.intersect(self.view_area.rect).is_empty();
            if !r {
                removed_items.push(*k);
            }
            r
        });
        removed_items
    }

    fn contains(&self, index: usize) -> bool {
        self.items.contains_key(&index)
    }
}

impl ListViewLayout {
    pub fn set_viewport_rect(&mut self, value: Rect) {
        self.view_area.rect = value;
    }

    pub fn get_index_range(&self) -> Range<usize> {
        (self.view_area.rect.min.y / self.item_size.y).floor() as usize
            ..(self.view_area.rect.max.y / self.item_size.y).ceil() as usize + 1
    }
}

pub fn list_bind_data<Item: DataItem>(
    mut set: ParamSet<(
        (
            &World,
            Query<(Entity, &ListViewLayout, One<&dyn ListViewModel<Item>>)>,
        ),
        Query<(
            Entity,
            &mut ListViewLayout,
            One<&mut dyn ListViewTrait>,
            One<&dyn ListItemViewFactory<Item>>,
        )>,
    )>,
    mut commands: Commands,
) {
    let (world, query) = set.p0();
    let mut update_list = vec![];
    for (container_entity, layout, model) in &query {
        let entity_ref = EntityWorldRef::new(world, container_entity);
        if model.update_from_world(entity_ref) {
            update_list.push((
                container_entity,
                model.get_changed(entity_ref),
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
            let item_layout = layout.add(commands.entity(container_entity), index);
            let entity = if let Some(item_entity) = view.get_entity(index) {
                item_entity
            } else {
                let entity = commands
                    .spawn(MiniNodeBundle {
                        node: Node {
                            top: Val::Px(item_layout.rect.min.y),
                            height: Val::Px(item_layout.rect.height()),
                            position_type: PositionType::Absolute,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .id();
                view.add(commands.entity(container_entity), index, entity);
                entity
            };
            item_factory.create_item(&index, commands.entity(entity), changed_item);
        }
        if let Some(range) = update_model.range {
            view.set_size(
                commands.entity(container_entity),
                Vec2::new(layout.item_size.x, layout.item_size.y * range.end as f32),
            );
        }
        for removed_index in layout.truncate(commands.reborrow()) {
            view.remove(commands.entity(container_entity), removed_index);
        }
    }
}

#[derive(Default)]
pub struct ListViewModelPlugin<Item: DataItem + Clone>(PhantomData<Item>);
impl<Item: DataItem + Clone> Plugin for ListViewModelPlugin<Item> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (list_bind_data::<Item>).in_set(UiFrameworkSystems::UpdateMVVM),
        );
    }
}
