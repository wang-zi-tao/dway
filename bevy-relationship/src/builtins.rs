use bevy::prelude::{Plugin, *};

use crate::{relationship, AppExt};

relationship!(IntersectWith => @both -< Intersect );

pub struct RelationshipPlugin;
impl Plugin for RelationshipPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.register_relation::<IntersectWith>();
    }
}
