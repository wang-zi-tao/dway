use std::iter::Cloned;

use bevy::prelude::{Children, Entity, Parent, Plugin};

use crate::{relationship, AppExt, Connectable, Peer, Relationship};

pub struct EntityHasChildren;
impl Relationship for EntityHasChildren {
    type From = Children;
    type To = Parent;
}
impl Connectable for Children {
    type Iterator<'l> = Cloned<std::slice::Iter<'l, Entity>>;

    fn iter<'l>(&'l self) -> Self::Iterator<'l> {
        (&**self).iter().cloned()
    }
}
impl Peer for Children {
    type Target = Parent;
}
pub struct ParentIter(pub Entity);
impl Iterator for ParentIter {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == Entity::PLACEHOLDER {
            None
        } else {
            Some(std::mem::replace(&mut self.0, Entity::PLACEHOLDER))
        }
    }
}
impl Connectable for Parent {
    type Iterator<'l> = ParentIter;

    fn iter<'l>(&'l self) -> Self::Iterator<'l> {
        ParentIter(self.get())
    }
}
impl Peer for Parent {
    type Target = Children;
}

relationship!(ReferenceTo => Reference -< ReferenceBy);
relationship!(IntersectWith => @both -< Intersect );

pub struct RelationshipPlugin;
impl Plugin for RelationshipPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_relation::<ReferenceTo>();
        app.register_relation::<IntersectWith>();
    }
}
