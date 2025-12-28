use bevy::{
    ecs::world::DeferredWorld,
    prelude::*,
};
use smallvec::SmallVec;

use crate::ConnectableMut;
pub fn disconnect_all_owned<ThisPeer: ConnectableMut, TargetPeer: ConnectableMut>(
    mut world: DeferredWorld,
    this_entity: Entity,
) {
    let peer_entitys = if let Some(mut from_component) = world.get_mut::<ThisPeer>(this_entity) {
        from_component.drain().collect::<SmallVec<[Entity; 8]>>()
    } else {
        Default::default()
    };
    for peer_entity in peer_entitys {
        if let Some(mut to_component) = world.get_mut::<TargetPeer>(peer_entity) {
            to_component.disconnect(this_entity);
            if to_component.as_slice().is_empty() {
                world.commands().entity(peer_entity).despawn();
            }
        }
    }
}

pub mod n_to_n {
    use bevy::{
        ecs::{
            component::{Mutable, StorageType},
            lifecycle::ComponentHook,
        },
        prelude::*,
    };

    use super::disconnect_all_owned;
    use crate::{relationship, Peer, Relationship};

    #[derive(Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct Reference(pub crate::RelationshipToManyEntity);

    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct Reference(crate::RelationshipToManyEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct Reference(crate::RelationshipToManyEntity)Connectable@peer(RcItem));
    relationship!(>-ReferenceFrom@peer(Reference));
    impl Peer for Reference {
        type Target = ReferenceFrom;
    }
    impl Peer for ReferenceFrom {
        type Target = Reference;
    }
    #[derive(Default)]
    pub struct ReferenceRelationship;

    impl Relationship for ReferenceRelationship {
        type From = Reference;
        type To = ReferenceFrom;
    }
    impl Component for Reference {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                disconnect_all_owned::<Reference, ReferenceFrom>(world, context.entity);
            })
        }
    }
    impl Component for ReferenceFrom {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                crate::disconnect_all::<ReferenceFrom, Reference>(world, context.entity);
            })
        }
    }
}

pub mod n_to_one {
    use bevy::{
        ecs::{
            component::{Mutable, StorageType},
            lifecycle::ComponentHook,
        },
        prelude::*,
    };

    use super::disconnect_all_owned;
    use crate::relationship;

    #[derive(Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct SharedRefreence(pub crate::RelationshipToOneEntity);

    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct SharedRefreence(crate::RelationshipToOneEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct SharedRefreence(crate::RelationshipToOneEntity)Connectable@peer(RcItem));
    relationship!(>-SharedReferenceFrom@peer(SharedRefreence));
    impl crate::Peer for SharedRefreence {
        type Target = SharedReferenceFrom;
    }
    impl crate::Peer for SharedReferenceFrom {
        type Target = SharedRefreence;
    }
    #[derive(Default)]
    pub struct SharedReferenceRelationship;

    impl crate::Relationship for SharedReferenceRelationship {
        type From = SharedRefreence;
        type To = SharedReferenceFrom;
    }
    impl Component for SharedRefreence {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                disconnect_all_owned::<SharedRefreence, SharedReferenceFrom>(world, context.entity);
            })
        }
    }
    impl Component for SharedReferenceFrom {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                crate::commands::disconnect_all::<SharedReferenceFrom, SharedRefreence>(
                    world,
                    context.entity,
                );
            })
        }
    }
}

pub mod one_to_one {
    use bevy::{
        ecs::{
            component::{Mutable, StorageType},
            lifecycle::ComponentHook,
        },
        prelude::*,
    };

    use super::disconnect_all_owned;
    use crate::relationship;

    #[derive(Clone, Default, Debug, crate::reexport::Reflect)]
    pub struct UniqueRefreence(pub crate::RelationshipToOneEntity);

    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct UniqueRefreence(crate::RelationshipToOneEntity)Deref@peer(RcItem));
    relationship!(#[derive(crate::reexport::Component,Clone,Default,Debug,crate::reexport::Reflect)]struct UniqueRefreence(crate::RelationshipToOneEntity)Connectable@peer(RcItem));
    relationship!(--UniqueReferenceFrom@peer(UniqueRefreence));
    impl crate::Peer for UniqueRefreence {
        type Target = UniqueReferenceFrom;
    }
    impl crate::Peer for UniqueReferenceFrom {
        type Target = UniqueRefreence;
    }
    #[derive(Default)]
    pub struct UniqueReferenceRelationship;

    impl crate::Relationship for UniqueReferenceRelationship {
        type From = UniqueRefreence;
        type To = UniqueReferenceFrom;
    }
    impl Component for UniqueRefreence {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                disconnect_all_owned::<UniqueRefreence, UniqueReferenceFrom>(world, context.entity);
            })
        }
    }
    impl Component for UniqueReferenceFrom {
        type Mutability = Mutable;

        const STORAGE_TYPE: StorageType = StorageType::Table;

        fn on_remove() -> Option<ComponentHook> {
            Some(|world, context| {
                crate::commands::disconnect_all::<UniqueReferenceFrom, UniqueRefreence>(
                    world,
                    context.entity,
                );
            })
        }
    }
}
