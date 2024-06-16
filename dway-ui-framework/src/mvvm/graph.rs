use bevy::ecs::system::EntityCommands;

use crate::prelude::*;

use super::{DataItem, DynEntityCommand, ViewFactory};

#[bevy_trait_query::queryable]
pub trait GraphViewModel<NodeId, Node: DataItem, Edge: DataItem> {
    fn iter(&self) -> Box<dyn Iterator<Item = (NodeId, Node)>>;
    fn node_out_edge(&self, node: &NodeId) -> Box<dyn Iterator<Item = (Edge, Node)>>;
    fn node_in_edge(&self, node: &NodeId) -> Box<dyn Iterator<Item = (Edge, Node)>>;
}

#[bevy_trait_query::queryable]
pub trait GraphItemViewFactory<NodeId, Node: DataItem, Edge: DataItem> {
    fn create_node(&self, index: &NodeId, commands: EntityCommands, item: Node);
    fn create_edge(&self, from: &NodeId, to: &NodeId, commands: EntityCommands, item: Edge);
}

impl<
        Node: DataItem,
        Edge: DataItem,
        NodeView: ViewFactory<Node>,
        EdgeView: ViewFactory<Edge>,
        NodeId: 'static,
    > GraphItemViewFactory<NodeId, Node, Edge> for (NodeView, EdgeView)
{
    fn create_node(&self, _index: &NodeId, commands: EntityCommands, item: Node) {
        ViewFactory::create(&self.0, commands, item)
    }
    fn create_edge(&self, _from: &NodeId, _to: &NodeId, commands: EntityCommands, item: Edge) {
        ViewFactory::create(&self.1, commands, item)
    }
}
