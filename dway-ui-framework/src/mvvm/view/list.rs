use std::collections::BTreeMap;

use bevy::{ecs::system::EntityCommands, reflect::Map};

use crate::{
    mvvm::{
        layout::ViewLayouter,
        list::{ListViewLayout, ListViewTrait},
    },
    prelude::*,
    widgets::scroll::{UiScroll, UiScrollState},
};

#[derive(Component, Default, Reflect)]
pub struct ListView {
    pub items: BTreeMap<usize, Entity>,
}

impl ListView {
    pub fn update_layout(
        mut query: Query<(&Self, &ComputedNode, &mut ListViewLayout, Ref<UiScrollState>)>,
        mut viewport_query: Query<&mut Node>,
    ) {
        for (this, computed_node, mut list_layout, scroll_state) in &mut query {
            if scroll_state.is_changed() {
                let rect = Rect::from_corners(
                    scroll_state.offset,
                    scroll_state.offset + scroll_state.size,
                );
                list_layout.set_view_rect(rect);
            }
            if scroll_state.is_added() {
                if let Some(mut node) = scroll_state
                    .content
                    .and_then(|e| viewport_query.get_mut(e).ok())
                {
                    node.flex_direction = FlexDirection::Column;
                }
            }
        }
    }
}

#[derive(Bundle)]
pub struct ListViewBundle {
    pub list_view: ListView,
    pub scroll: UiScrollBundle,
    pub node: Node,
}

impl Default for ListViewBundle {
    fn default() -> Self {
        Self {
            list_view: Default::default(),
            node: style!("full"),
            scroll: UiScrollBundle {
                prop: UiScroll {
                    horizontal: false,
                    vertical: true,
                    create_viewport: true,
                },
                ..Default::default()
            },
        }
    }
}

impl ListViewTrait for ListView {
    fn add(&mut self, mut commands: EntityCommands, item_index: usize, item_view_entity: Entity) {
        self.items.insert(item_index, item_view_entity);
        commands.queue(move |c: EntityWorldMut<'_>| {
            if let Some(content_entity) =
                c.get::<UiScrollState>().and_then(|state| *state.content())
            {
                c.into_world_mut()
                    .entity_mut(content_entity)
                    .add_child(item_view_entity);
            }
        });
    }

    fn remove(&mut self, mut commands: EntityCommands, item_index: usize) {
        if let Some(entity) = self.items.remove(&item_index) {
            commands.commands().entity(entity).despawn_recursive();
        }
    }

    fn get_entity(&self, item_index: usize) -> Option<Entity> {
        self.items.get(&item_index).cloned()
    }

    fn set_size(&mut self, mut commands: EntityCommands, size: Vec2) {
        commands.queue(move |c: EntityWorldMut<'_>| {
            if let Some(mut style) = c
                .get::<UiScrollState>()
                .and_then(|state| *state.content())
                .and_then(|content_entity| c.into_world_mut().get_mut::<Node>(content_entity))
            {
                style.width = Val::Px(size.x);
                style.height = Val::Px(size.y);
            }
        });
    }
}
