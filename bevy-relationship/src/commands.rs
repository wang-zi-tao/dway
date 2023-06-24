use std::marker::PhantomData;

use bevy::{
    ecs::system::Command,
    prelude::{Entity, World},
};
use smallvec::SmallVec;

use crate::{Connectable, ConnectableMut, Relationship};

#[derive(Debug)]
pub struct ConnectCommand<R: Relationship> {
    pub from: Entity,
    pub to: Entity,
    pub phantom_data: PhantomData<R>,
}

impl<R: Relationship> ConnectCommand<R> {
    pub fn new(from: Entity, to: Entity) -> Self {
        Self {
            from,
            to,
            phantom_data: PhantomData,
        }
    }
}
impl<R: Relationship + Send + Sync + 'static> Command for ConnectCommand<R>
where
    R::From: ConnectableMut + Default,
    R::To: ConnectableMut + Default,
{
    fn write(self, world: &mut World) {
        if world.get_entity(self.to).is_none() {
            return;
        };
        let Some( mut from_entity ) = world.get_entity_mut(self.from)else{return};
        let old = if let Some(mut peer) = from_entity.get_mut::<R::From>() {
            peer.connect(self.to)
        } else {
            let mut peer = R::From::default();
            let old = peer.connect(self.to);
            from_entity.insert(peer);
            old
        };
        if let Some(old) = old {
            if let Some(mut old_component) = world.get_mut::<R::To>(old) {
                old_component.disconnect(self.from);
            }
        }

        let Some( mut to_entity ) = world.get_entity_mut(self.to)else{return};
        let old = if let Some(mut peer) = to_entity.get_mut::<R::To>() {
            peer.connect(self.from)
        } else {
            let mut peer = R::To::default();
            let old = peer.connect(self.from);
            to_entity.insert(peer);
            old
        };
        if let Some(old) = old {
            if let Some(mut old_component) = world.get_mut::<R::From>(old) {
                old_component.disconnect(self.to);
            }
        }
    }
}

pub struct DisconnectCommand<R: Relationship> {
    pub from: Entity,
    pub to: Entity,
    _phantom: PhantomData<R>,
}

impl<R: Relationship> DisconnectCommand<R> {
    pub fn new(from: Entity, to: Entity) -> Self {
        Self {
            from,
            to,
            _phantom: PhantomData,
        }
    }
}
impl<R: Relationship + Sync + Send + 'static> Command for DisconnectCommand<R>
where
    R::From: ConnectableMut,
    R::To: ConnectableMut,
{
    fn write(self, world: &mut World) {
        if let Some(mut component) = world.get_mut::<R::From>(self.from) {
            component.disconnect(self.to);
        }

        if let Some(mut component) = world.get_mut::<R::To>(self.to) {
            component.disconnect(self.from);
        }
    }
}
pub struct DisconnectAllCommand<R: Relationship> {
    pub from: Entity,
    _phantom: PhantomData<R>,
}

impl<R: Relationship> DisconnectAllCommand<R> {
    pub fn new(from: Entity) -> Self {
        Self {
            from,
            _phantom: PhantomData,
        }
    }
}
impl<R: Relationship + Send + Sync + 'static> Command for DisconnectAllCommand<R>
where
    R::From: ConnectableMut,
    R::To: ConnectableMut,
{
    fn write(self, world: &mut World) {
        let mut from_query = world.query::<&mut R::From>();
        let mut to_query = world.query::<&mut R::To>();
        let target = if let Ok(mut from_component) = from_query.get_mut(world, self.from) {
            from_component.drain().collect::<SmallVec<[Entity; 8]>>()
        } else {
            Default::default()
        };
        for entity in target {
            if let Ok(mut to_component) = to_query.get_mut(world, entity) {
                to_component.disconnect(self.from);
            }
        }
    }
}

#[derive(Default)]
pub struct ReverseRelationship<T>(T);

impl<T> Relationship for ReverseRelationship<T>
where
    T: Relationship,
{
    type From = T::To;

    type To = T::From;
}
