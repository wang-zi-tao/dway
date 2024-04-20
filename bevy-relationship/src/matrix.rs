use bevy::{
    ecs::{component::Tick, entity::EntityHashMap},
    prelude::*,
};
use petgraph::{csr, Directed};

use crate::Relationship;

pub trait MRelationship {
    type Weight;
}

pub struct EdgeMatrix<R: MRelationship> {
    pub node_tick: EntityHashMap<(Tick, Tick)>,
    pub matrix: csr::Csr<Entity, (Tick, R::Weight)>,
}
