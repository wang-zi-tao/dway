use bevy::prelude::*;

use crate::ConnectableMut;
fn despawn_peer_if_empty<C: ConnectableMut>(
    world: &mut World,
    this_entity: Entity,
    peer_entity: Entity,
) {
    if let Some(mut component) = world.get_mut::<C>(peer_entity) {
        component.disconnect(this_entity);
        if component.as_slice().is_empty() {
            world.entity_mut(peer_entity).despawn_recursive();
        }
    };
}

pub mod n_to_n {
    use bevy::prelude::*;

    use super::despawn_peer_if_empty;
    use crate::relationship;

    #[derive(crate::reexport::Component, Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct Reference(pub crate::RelationshipToManyEntity);

    impl Drop for Reference {
        fn drop(&mut self) {
            for peer_entity in crate::Connectable::iter(self) {
                self.sender
                    .send_function(peer_entity, despawn_peer_if_empty::<ReferenceFrom>);
            }
        }
    }
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct Reference(crate::RelationshipToManyEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct Reference(crate::RelationshipToManyEntity)Connectable@peer(RcItem));
    relationship!(>-ReferenceFrom@peer(Reference));
    relationship!(@Relationship ReferenceRelationship => Reference-ReferenceFrom);
}

pub mod n_to_one {
    use bevy::prelude::*;

    use super::despawn_peer_if_empty;
    use crate::relationship;

    #[derive(crate::reexport::Component, Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct SharedRefreence(pub crate::RelationshipToOneEntity);

    impl Drop for SharedRefreence {
        fn drop(&mut self) {
            for peer_entity in crate::Connectable::iter(self) {
                self.sender
                    .send_function(peer_entity, despawn_peer_if_empty::<SharedReferenceFrom>);
            }
        }
    }
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct SharedRefreence(crate::RelationshipToOneEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct SharedRefreence(crate::RelationshipToOneEntity)Connectable@peer(RcItem));
    relationship!(>-SharedReferenceFrom@peer(SharedRefreence));
    relationship!(@Relationship SharedReferenceRelationship => SharedRefreence-SharedReferenceFrom);
}

pub mod one_to_one {
    use bevy::prelude::*;

    use super::despawn_peer_if_empty;
    use crate::relationship;

    #[derive(crate::reexport::Component, Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct UniqueRefreence(pub crate::RelationshipToOneEntity);

    impl Drop for UniqueRefreence {
        fn drop(&mut self) {
            for peer_entity in crate::Connectable::iter(self) {
                self.sender
                    .send_function(peer_entity, despawn_peer_if_empty::<UniqueReferenceFrom>);
            }
        }
    }
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct UniqueRefreence(crate::RelationshipToOneEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct UniqueRefreence(crate::RelationshipToOneEntity)Connectable@peer(RcItem));
    relationship!(--UniqueReferenceFrom@peer(UniqueRefreence));
    relationship!(@Relationship UniqueReferenceRelationship => UniqueRefreence-UniqueReferenceFrom);
}
