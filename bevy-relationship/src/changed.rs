use bevy::{ecs::entity::EntityHashMap, prelude::*};
use smallvec::SmallVec;

pub trait GraphQueryCache{
    type C: Component;
    type I: Iterator<Item = Entity>;

    fn iter(&self) -> Self::I;

    fn is_changed(&self, component: &Self::C)->bool;
}

pub struct BeginNodeCache{
    
}

pub struct NodeCache<Path>{
    pub entitys: EntityHashMap<SmallVec<[Path;1]>>,
}
