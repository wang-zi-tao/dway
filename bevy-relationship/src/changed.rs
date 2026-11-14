use bevy::{ecs::entity::EntityHashMap, prelude::*};
use smallvec::SmallVec;

#[allow(dead_code)]
pub trait GraphQueryCache {
    type C: Component;
    type I: Iterator<Item = Entity>;

    fn iter(&self) -> Self::I;

    fn is_changed(&self, component: &Self::C) -> bool;
}

#[allow(dead_code)]
pub struct BeginNodeCache {}

#[allow(dead_code)]
pub struct NodeCache<Path> {
    pub entitys: EntityHashMap<SmallVec<[Path; 1]>>,
}
