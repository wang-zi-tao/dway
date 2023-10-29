use std::{cell::Ref, marker::PhantomData};

use bevy::{
    ecs::{
        component::ComponentId,
        system::{Command, EntityCommands},
        world::EntityMut,
    },
    prelude::{despawn_with_children_recursive, BuildWorldChildren, Children, Entity, World, debug},
    utils::HashMap,
};
use smallvec::SmallVec;

use crate::{ConnectableMut, Relationship, RelationshipRegister, ReserveRelationship};

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
    fn apply(self, world: &mut World) {
        if world.get_entity(self.to).is_none() {
            return;
        };
        let Some(mut from_entity) = world.get_entity_mut(self.from) else {
            return;
        };
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

        let Some(mut to_entity) = world.get_entity_mut(self.to) else {
            return;
        };
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
    fn apply(self, world: &mut World) {
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
    fn apply(self, world: &mut World) {
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

pub struct DespawnAllConnectedEntityCommand<R: Relationship> {
    pub from: Entity,
    _phantom: PhantomData<R>,
}

impl<R: Relationship> DespawnAllConnectedEntityCommand<R> {
    pub fn new(from: Entity) -> Self {
        Self {
            from,
            _phantom: PhantomData,
        }
    }
}

impl<R: Relationship + Send + Sync + 'static> Command for DespawnAllConnectedEntityCommand<R>
where
    R::From: ConnectableMut,
    R::To: ConnectableMut,
{
    fn apply(self, world: &mut World) {
        let mut from_query = world.query::<&mut R::From>();
        let target = if let Ok(mut from_component) = from_query.get_mut(world, self.from) {
            from_component.drain().collect::<SmallVec<[Entity; 8]>>()
        } else {
            Default::default()
        };
        for entity in target {
            despawn_with_children_recursive(world, entity);
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

pub struct DespawnRecursiveCommand(pub Entity);
impl Command for DespawnRecursiveCommand {
    fn apply(self, world: &mut World) {
        debug!("Despawning entity {:?}",self.0);
        despawn_recursive(world, self.0);
    }
}

pub struct DespawnCommand(pub Entity);
impl Command for DespawnCommand {
    fn apply(self, world: &mut World) {
        despawn(world, self.0);
    }
}

pub fn despawn_recursive(world: &mut World, entity: Entity) {
    let register = world
        .non_send_resource::<RelationshipRegister>()
        .components
        .clone();
    let guard = register.borrow();
    world.get_entity_mut(entity).map(|mut e| {
        e.remove_parent();
    });
    do_despawn_recursive(world, entity, &guard);
}

pub fn despawn(world: &mut World, entity: Entity) {
    let register = world
        .non_send_resource::<RelationshipRegister>()
        .components
        .clone();
    let guard = register.borrow();
    if let Some(entity_mut) = world.get_entity_mut(entity) {
        do_despawn(entity_mut, &guard);
    }
}

fn do_despawn_recursive(
    world: &mut World,
    entity: Entity,
    register: &Ref<'_, HashMap<ComponentId, Box<dyn Fn(&mut EntityMut<'_>) + Send + Sync>>>,
) {
    if let Some(children) = world.get_mut::<Children>(entity) {
        let children_entitys = children.iter().copied().collect::<SmallVec<[Entity; 63]>>();
        drop(children);
        for child in children_entitys {
            do_despawn_recursive(world, child, register);
        }
    }
    if let Some(entity_mut) = world.get_entity_mut(entity) {
        do_despawn(entity_mut, register);
    }
}

fn do_despawn(
    mut entity_mut: EntityMut,
    register: &Ref<'_, HashMap<ComponentId, Box<dyn Fn(&mut EntityMut<'_>) + Send + Sync>>>,
) {
    for component_id in entity_mut
        .archetype()
        .table_components()
        .collect::<SmallVec<[ComponentId; 31]>>()
    {
        if let Some(callback) = register.get(&component_id) {
            let _ = callback(&mut entity_mut);
        }
    }
    entity_mut.despawn();
}

pub trait EntityCommandsExt {
    fn despawn_with_relationship(self);
    fn despawn_recursive_with_relationship(self);
    fn connect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn connect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_all<R: Relationship + Send + Sync + 'static>(&mut self)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_all_rev<R: Relationship + Send + Sync + 'static>(&mut self)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
}

impl<'w, 's, 'a> EntityCommandsExt for EntityCommands<'w, 's, 'a> {
    fn despawn_with_relationship(mut self) {
        let entity = self.id();
        self.commands().add(DespawnCommand(entity));
    }

    fn despawn_recursive_with_relationship(mut self) {
        let entity = self.id();
        self.commands().add(DespawnRecursiveCommand(entity));
    }

    fn connect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(ConnectCommand::<R>::new(entity, peer));
    }

    fn connect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(ConnectCommand::<R>::new(peer, entity));
    }

    fn disconnect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectCommand::<R>::new(entity, peer));
    }

    fn disconnect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectCommand::<R>::new(peer, entity));
    }

    fn disconnect_all<R: Relationship + Send + Sync + 'static>(&mut self)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(DisconnectAllCommand::<R>::new(entity));
    }

    fn disconnect_all_rev<R: Relationship + Send + Sync + 'static>(&mut self)
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectAllCommand::<ReserveRelationship<R>>::new(entity));
    }
}
