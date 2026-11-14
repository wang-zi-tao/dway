use bevy::{
    ecs::{change_detection::Tick, entity::EntityHashMap},
    prelude::*,
};
use petgraph::csr;

#[allow(dead_code)]
pub trait MRelationship {
    type Weight;
}

#[allow(dead_code)]
pub struct EdgeMatrix<R: MRelationship> {
pub node_tick: EntityHashMap<(Tick, Tick)>,
pub matrix: csr::Csr<Entity, (Tick, R::Weight)>,
}
