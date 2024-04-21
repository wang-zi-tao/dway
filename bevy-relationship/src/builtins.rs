use std::{iter::Cloned, ops::Deref};

use bevy::prelude::{Children, Entity, Parent, Plugin};

use crate::{relationship, AppExt, Connectable, Peer, Relationship};

relationship!(ReferenceTo => Reference -< ReferenceBy);
relationship!(IntersectWith => @both -< Intersect );

pub struct RelationshipPlugin;
impl Plugin for RelationshipPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_relation::<ReferenceTo>();
        app.register_relation::<IntersectWith>();
    }
}
