mod app;
mod builtins;
mod commands;
mod macros;
pub mod reexport;

pub use bevy_relationship_derive::graph_query;

use std::{
    iter::{Cloned, Peekable},
    marker::PhantomData,
};

pub use crate::{app::*, builtins::*, commands::*, macros::*};

use bevy::ecs::{
    query::{QueryIter, ROQueryItem, ReadOnlyWorldQuery, WorldQuery},
    system::{Command, SystemParam},
};

use bevy::prelude::*;
use smallvec::SmallVec;

#[derive(Component, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Default, Debug, Reflect)]
#[reflect(Debug)]
pub struct RelationshipToOneEntity(pub Option<Entity>);
impl RelationshipToOneEntity {
    pub fn get(&self) -> Option<Entity> {
        self.0
    }
    pub fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.0.replace(entity)
    }
    pub fn take(&mut self) -> Option<Entity> {
        self.0.take()
    }
}
impl Connectable for RelationshipToOneEntity {
    type Iterator<'l> = Cloned<std::option::Iter<'l, Entity>>;

    fn iter(&self) -> Self::Iterator<'_> {
        self.0.iter().cloned()
    }
}
impl ConnectableMut for RelationshipToOneEntity {
    type Drain<'l> = std::option::IntoIter<Entity>;

    fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.0.replace(entity)
    }

    fn disconnect(&mut self, target: Entity) -> bool {
        if let Some(entity) = self.0 {
            if entity == target {
                self.0.take();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn drain(&mut self) -> Self::Drain<'_> {
        self.0.take().into_iter()
    }
}
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Debug)]
pub struct RelationshipToManyEntity(pub SmallVec<[Entity; 4]>);
impl Connectable for RelationshipToManyEntity {
    type Iterator<'l> = Cloned<std::slice::Iter<'l, Entity>>;

    fn iter(&self) -> Self::Iterator<'_> {
        self.0.iter().cloned()
    }
}
impl ConnectableMut for RelationshipToManyEntity {
    type Drain<'l> = smallvec::Drain<'l, [Entity; 4]>;

    fn connect(&mut self, entity: Entity) -> Option<Entity> {
        self.0.push(entity);
        None
    }

    fn disconnect(&mut self, target: Entity) -> bool {
        if let Some(index) = self.0.iter().position(|entity| target == *entity) {
            self.0.swap_remove(index);
            true
        } else {
            false
        }
    }

    fn drain(&mut self) -> Self::Drain<'_> {
        self.0.drain(..)
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
    fn iter<'l>(&'l self) -> Self::Iterator<'l>;
}
pub trait ConnectableMut: Connectable {
    type Drain<'l>: Iterator<Item = Entity>;
    fn connect(&mut self, target: Entity) -> Option<Entity>;
    fn disconnect(&mut self, target: Entity) -> bool;
    fn drain<'l>(&'l mut self) -> Self::Drain<'l>;
}

