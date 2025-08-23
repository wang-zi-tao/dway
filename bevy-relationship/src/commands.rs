use std::marker::PhantomData;

use bevy::{
    ecs::{
        system::EntityCommands,
        world::{DeferredWorld},
    },
    prelude::{Entity, World, Command},
};
use smallvec::SmallVec;

use crate::{ConnectableMut, Relationship, ReserveRelationship};

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
        if world.get_entity(self.to).is_err() {
            return;
        };
        let Ok(mut from_entity) = world.get_entity_mut(self.from) else {
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

        let Ok(mut to_entity) = world.get_entity_mut(self.to) else {
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

pub fn disconnect_all<ThisPeer: ConnectableMut, TargetPeer: ConnectableMut>(
    mut world: DeferredWorld,
    this_entity: Entity,
) {
    let peer_entitys = if let Some(mut out_component) = world.get_mut::<ThisPeer>(this_entity) {
        out_component.drain().collect::<SmallVec<[Entity; 8]>>()
    } else {
        Default::default()
    };
    for peer_entity in peer_entitys {
        if let Some(mut in_component) = world.get_mut::<TargetPeer>(peer_entity) {
            in_component.disconnect(this_entity);
        }
    }
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
        disconnect_all::<R::From, R::To>(world.into(), self.from);
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
            world.despawn(entity);
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
    fn disconnect_from<R: Relationship + Send + Sync + 'static>(
        &mut self,
        peer: Entity,
    ) -> &mut Self
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

impl<'w> EntityCommandsExt for EntityCommands<'w> {
    fn connect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().queue(ConnectCommand::<R>::new(entity, peer));
        self
    }

    fn connect_from<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().queue(ConnectCommand::<R>::new(peer, entity));
        self
    }

    fn disconnect_to<R: Relationship + Send + Sync + 'static>(&mut self, peer: Entity) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .queue(DisconnectCommand::<R>::new(entity, peer));
        self
    }

    fn disconnect_from<R: Relationship + Send + Sync + 'static>(
        &mut self,
        peer: Entity,
    ) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .queue(DisconnectCommand::<R>::new(peer, entity));
        self
    }

    fn disconnect_all<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands().queue(DisconnectAllCommand::<R>::new(entity));
        self
    }

    fn disconnect_all_rev<R: Relationship + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let entity = self.id();
        self.commands()
            .queue(DisconnectAllCommand::<ReserveRelationship<R>>::new(entity));
        self
    }
}
