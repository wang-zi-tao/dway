use std::marker::PhantomData;
use bevy::{
    ecs::system::{Command, EntityCommands},
    prelude::{
        despawn_with_children_recursive, Entity,
        EntityWorldMut, World,
    },
};
use smallvec::SmallVec;
use crate::{
    ConnectableMut, ConnectionEventReceiver, ConnectionEventSender, Relationship,
    ReserveRelationship,
};

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
        fn get_sender(entity_mut: &mut EntityWorldMut, entity: Entity) -> ConnectionEventSender {
            entity_mut.world_scope(|world| {
                world
                    .non_send_resource::<ConnectionEventReceiver>()
                    .get_sender(entity)
            })
        }

        if world.get_entity(self.to).is_none() {
            return;
        };
        let Some(mut from_entity) = world.get_entity_mut(self.from) else {
            return;
        };
        let old = if let Some(mut peer) = from_entity.get_mut::<R::From>() {
            let old = peer.connect(self.to);
            if !peer.get_sender_mut().inited() {
                let new_sender = get_sender(&mut from_entity, self.from);
                let mut peer = from_entity.get_mut::<R::From>().unwrap();
                *peer.get_sender_mut() = new_sender;
            }
            old
        } else {
            let mut peer = R::From::default();
            *peer.get_sender_mut() = get_sender(&mut from_entity, self.from);
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
            let old = peer.connect(self.from);
            if !peer.get_sender_mut().inited() {
                let new_sender = get_sender(&mut to_entity, self.to);
                let mut peer = to_entity.get_mut::<R::To>().unwrap();
                *peer.get_sender_mut() = new_sender;
            }
            old
        } else {
            let mut peer = R::To::default();
            *peer.get_sender_mut() = get_sender(&mut to_entity, self.to);
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

pub trait EntityCommandsExt {
    fn connect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn connect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_all<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
    fn disconnect_all_rev<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default;
}

impl<'w, 's, 'a> EntityCommandsExt for EntityCommands<'w, 's, 'a> {
    fn connect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(ConnectCommand::<R>::new(entity, peer));
        self
    }

    fn connect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(ConnectCommand::<R>::new(peer, entity));
        self
    }

    fn disconnect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectCommand::<R>::new(entity, peer));
        self
    }

    fn disconnect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectCommand::<R>::new(peer, entity));
        self
    }

    fn disconnect_all<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().add(DisconnectAllCommand::<R>::new(entity));
        self
    }

    fn disconnect_all_rev<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .add(DisconnectAllCommand::<ReserveRelationship<R>>::new(entity));
        self
    }
}
