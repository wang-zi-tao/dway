use std::{cell::RefCell, rc::Rc};

use bevy::{
    ecs::{component::ComponentId, world::EntityMut},
    prelude::*,
    utils::HashMap,
};
use smallvec::SmallVec;

use crate::{ConnectableMut, Relationship, ReverseRelationship};

#[derive(Default)]
pub struct RelationshipRegister {
    pub(crate) components:
        Rc<RefCell<HashMap<ComponentId, Box<dyn Fn(&mut EntityMut<'_>) + Send + Sync>>>>,
}

impl RelationshipRegister {
    pub fn register<R: Relationship + 'static>(world: &mut World)
where
    R::From: ConnectableMut,
    R::To: ConnectableMut,
    {
        let from_component_id = world.component_id::<R::From>().unwrap_or_else(||world.init_component::<R::From>());
        let to_component_id = world.component_id::<R::To>().unwrap_or_else(||world.init_component::<R::To>());

        if !world.contains_non_send::<Self>(){
            world.insert_non_send_resource(Self::default());
        }
        let this = world.non_send_resource_mut::<Self>();

        let mut guard = this.components.borrow_mut();
        guard.insert(from_component_id, Box::new( remove_peer::<R> ));
        guard.insert(to_component_id, Box::new( remove_peer::<ReverseRelationship<R>> ));
    }
}

fn remove_peer<R: Relationship>(entity_mut: &mut EntityMut)
where
    R::From: ConnectableMut,
    R::To: ConnectableMut,
{
    let entity = entity_mut.id();
    if let Some(mut component) = entity_mut.get_mut::<R::From>() {
        for target in component.drain().collect::<SmallVec<[Entity; 63]>>() {
            if target != entity {
                entity_mut.world_scope(|world| {
                    world
                        .get_mut::<R::To>(target)
                        .map(|mut c| c.disconnect(entity));
                })
            }
        }
    };
}
