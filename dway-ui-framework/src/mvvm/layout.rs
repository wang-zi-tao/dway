use std::marker::PhantomData;

use bevy::{ecs::system::EntityCommands, platform::collections::HashMap};
use slab_tree::Tree;

use crate::prelude::*;

pub trait ViewLayouter<Index> {
    fn contains(&self, index: Index) -> bool;
    fn add(&mut self, entity: EntityCommands, index: Index) -> ItemLayout;
    fn remove(&mut self, entity: EntityCommands, index: Index);
    fn get_item_layout(&self, index: Index) -> Option<&ItemLayout>;
    fn get_item_layout_mut(&mut self, index: Index) -> Option<&mut ItemLayout>;
    fn set_view_rect(&mut self, rect: Rect);
    fn truncate(&mut self, commands: Commands) -> Vec<Index>;
}

#[derive(Clone, Reflect, Debug, Default)]
pub struct ItemLayout {
    pub rect: Rect,
}

#[derive(Reflect, Default, Clone)]
pub struct ViewAreaLayout {
    pub rect: Rect,
}

pub struct ContainerViewLayout<Index> {
    pub default_item_size: Vec2,
    pub items: HashMap<Index, ItemLayout>,
    pub view_area: ViewAreaLayout,
}

pub enum LayoutChange<Index> {
    Add {
        index: Index,
        layout: ItemLayout,
    },
    Update {
        index: Index,
        old_layout: ItemLayout,
        new_layout: ItemLayout,
    },
    Remove {
        index: Index,
        layout: ItemLayout,
    },
}

pub struct HeaderCell {
    pub position: f32,
    pub size: f32,
    pub pin: bool,
    pub resizable: bool,
}

pub struct HonHeader;
pub struct VerHeader;
pub struct Header<Direction> {
    pub size: Vec<f32>,
    phantom: PhantomData<Direction>,
}

pub struct ItemLayoutFromHeader<Direction> {
    pub header_entity: Entity,
    phantom: PhantomData<Direction>,
}

structstruck::strike! {
    pub struct TreeView<NodeId>{
        pub item_size: Vec2,
        pub view_area: ViewAreaLayout,
        pub items: Tree<struct TreeViewItem<NodeId> {
            id: NodeId,
            layout: ItemLayout,
            is_folded: bool,
        }>,
    }
}

structstruck::strike! {
    pub struct GraphView<NodeId>{
        pub item_size: Vec2,
        pub view_area: ViewAreaLayout,
        pub items: petgraph::Graph<
            struct GraphViewNode<NodeId> {
                id: NodeId,
                layout: ItemLayout,
                is_folded: bool,
            },
            struct GraphViewEdge<NodeId>{
                from_node: NodeId,
                to_node: NodeId,
            }
        >,
    }
}
