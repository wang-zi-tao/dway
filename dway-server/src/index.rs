use bevy::{prelude::*, utils::HashMap};
use smithay::reexports::wayland_server::backend::smallvec::SmallVec;

// pub struct Index(Vec<Entity>);

pub struct NewEntity {}

pub trait Index<V: Component> {
    fn add(entity: Entity, value: &V) -> bool;
    fn remove(entity: Entity) -> bool;
}

pub struct Indexed(Entity);

pub fn update_index_on_add() {}
pub fn update_index_on_remove<E: Component>(
    mut query: RemovedComponents<E>,
    index_map: ResMut<IndexMap>,
) {
    for entity in query.into_iter() {}
}
#[derive(Resource)]
pub struct IndexMap {
    map: HashMap<Entity, SmallVec<[Entity; 1]>>,
}
