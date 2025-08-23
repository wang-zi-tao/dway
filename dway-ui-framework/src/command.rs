use bevy::ecs::system::{SystemBuffer, SystemParam, SystemState};

pub use crate::prelude::*;

#[bevy_trait_query::queryable]
pub trait DestroyInterceptor {
    fn apply(&self, entity: &EntityRef, commands: Commands) -> bool;
}

pub fn destroy_ui(entity: EntityWorldMut) {
    let entity_id = entity.id();
    let world = entity.into_world_mut();
    let mut param = SystemState::<(Query<&dyn DestroyInterceptor>, Commands)>::new(world);
    let (query, mut commands) = param.get_manual(world);

    let Ok(entity_ref) = world.get_entity(entity_id) else {
        return;
    };

    let mut despawn = true;
    for component in query.get(entity_id).ok().iter().flatten() {
        if component.apply(&entity_ref, commands.reborrow()) {
            despawn = false;
        }
    }

    if despawn {
        if let Ok(entity_mut) = world.get_entity_mut(entity_id) {
            entity_mut.despawn();
        }
    }
}

pub fn destroy_children_ui(entity: EntityWorldMut) {
    let entity_id = entity.id();
    let world = entity.into_world_mut();
    let Some(children) = world.get::<Children>(entity_id) else {
        return;
    };
    let children_vec = children.iter().collect::<Vec<_>>();
    for child in children_vec {
        destroy_ui(world.entity_mut(child));
    }
}
