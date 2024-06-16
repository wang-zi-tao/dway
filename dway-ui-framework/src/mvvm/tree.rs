use crate::prelude::*;

use super::{ViewFactory, ContainerViewFactory, DataItem, DynEntityCommand};

#[bevy_trait_query::queryable]
pub trait TreeViewModel<NodeId, Item: DataItem> {
    fn get_root(&self) -> Box<dyn Iterator<Item = (NodeId, Item)>>;
    fn get_children(&self, node: &NodeId) -> Box<dyn Iterator<Item = (NodeId, Item)>>;
}

#[bevy_trait_query::queryable]
pub trait TreeItemViewFactory<NodeId, Item: DataItem>: ContainerViewFactory<NodeId,Item> {
}

impl<Item: DataItem, T: ViewFactory<Item> + ContainerViewFactory<NodeId, Item>, NodeId: 'static> TreeItemViewFactory<NodeId, Item> for T {
}
