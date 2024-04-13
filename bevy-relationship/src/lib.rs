mod app;
mod builtins;
mod commands;
mod graph;
mod macros;
pub mod reexport;
mod register;
mod changed;

#[allow(unused_imports)]
pub use crate::{app::*, builtins::*, commands::*, graph::*, macros::*, register::*};
pub use app::AppExt;
use bevy::prelude::*;
pub use bevy_relationship_derive::graph_query;
use smallvec::SmallVec;
use std::{iter::Cloned, marker::PhantomData};

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Debug)]
#[derive(Default)]
pub struct RelationshipToOneEntity {
    pub peer: Option<Entity>,
    #[reflect(ignore)]
    pub sender: ConnectionEventSender,
}


impl RelationshipToOneEntity {
    pub fn get(&self) -> Option<Entity> {
        self.peer
    }
    pub fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.peer.replace(entity)
    }
    pub fn take(&mut self) -> Option<Entity> {
        self.peer.take()
    }
}
impl Connectable for RelationshipToOneEntity {
    type Iterator<'l> = Cloned<std::option::Iter<'l, Entity>>;

    fn iter(&self) -> Self::Iterator<'_> {
        self.peer.iter().cloned()
    }
}
impl ConnectableMut for RelationshipToOneEntity {
    type Drain<'l> = std::option::IntoIter<Entity>;

    fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.peer.replace(entity)
    }

    fn disconnect(&mut self, target: Entity) -> bool {
        if let Some(entity) = self.peer {
            if entity == target {
                self.peer.take();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn drain(&mut self) -> Self::Drain<'_> {
        self.peer.take().into_iter()
    }

    fn get_sender_mut(&mut self) -> &mut ConnectionEventSender {
        &mut self.sender
    }
}
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Debug)]
#[derive(Default)]
pub struct RelationshipToManyEntity {
    pub peers: SmallVec<[Entity; 4]>,
    #[reflect(ignore)]
    pub sender: ConnectionEventSender,
}


impl std::ops::Deref for RelationshipToManyEntity {
    type Target = SmallVec<[Entity; 4]>;

    fn deref(&self) -> &Self::Target {
        &self.peers
    }
}
impl Connectable for RelationshipToManyEntity {
    type Iterator<'l> = Cloned<std::slice::Iter<'l, Entity>>;

    fn iter(&self) -> Self::Iterator<'_> {
        self.peers.iter().cloned()
    }
}
impl ConnectableMut for RelationshipToManyEntity {
    type Drain<'l> = smallvec::Drain<'l, [Entity; 4]>;

    fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.peers.push(entity);
        None
    }

    fn disconnect(&mut self, target: Entity) -> bool {
        if let Some(index) = self.peers.iter().position(|entity| target == *entity) {
            self.peers.swap_remove(index);
            true
        } else {
            false
        }
    }

    fn drain(&mut self) -> Self::Drain<'_> {
        self.peers.drain(..)
    }

    fn get_sender_mut(&mut self) -> &mut ConnectionEventSender {
        &mut self.sender
    }
}
pub trait Peer: Connectable {
    type Target: Peer<Target = Self>;
}
pub trait Relationship {
    type From: Peer<Target = Self::To>;
    type To: Peer<Target = Self::From>;
}
pub struct ReserveRelationship<T>(pub PhantomData<T>);
impl<T: Relationship> Relationship for ReserveRelationship<T> {
    type From = T::To;
    type To = T::From;
}
pub trait Connectable: Component {
    type Iterator<'l>: Iterator<Item = Entity>;
    fn iter(&self) -> Self::Iterator<'_>;

    fn contains(&self, entity: Entity) -> bool {
        self.iter().any(|e| e == entity)
    }
}
pub trait ConnectableMut: Connectable {
    type Drain<'l>: Iterator<Item = Entity>;
    fn connect(&mut self, target: Entity) -> Option<Entity>;
    fn disconnect(&mut self, target: Entity) -> bool;
    fn drain(&mut self) -> Self::Drain<'_>;

    fn get_sender_mut(&mut self) -> &mut ConnectionEventSender;
}
