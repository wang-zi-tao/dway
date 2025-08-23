#[macro_export]
macro_rules! relationship {
    (#[derive $derive:tt] struct $name:ident($inner:ty) Deref @peer($peer:ty)) => {
        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };
    (#[derive $derive:tt] struct $name:ident($inner:ty) Connectable @peer($peer:ty)) => {
        impl $crate::Connectable for $name {
            type Iterator<'l> = <$inner as $crate::Connectable>::Iterator<'l>;

            fn iter(&self) -> Self::Iterator<'_> {
                self.0.iter()
            }

            fn as_slice(&self) -> &[Entity] {
                self.0.as_slice()
            }
        }
        impl $crate::ConnectableMut for $name {
            type Drain<'l> = <$inner as $crate::ConnectableMut>::Drain<'l>;

            fn connect(&mut self, target: $crate::reexport::Entity) -> Option<$crate::reexport::Entity> {
                self.0.connect(target)
            }

            fn disconnect(&mut self, target: $crate::reexport::Entity) -> bool {
                self.0.disconnect(target)
            }

            fn drain(&mut self) -> Self::Drain<'_> {
                self.0.drain()
            }
        }
    };

    (#[derive $derive:tt] struct $name:ident($inner:ty) @peer($peer:ty)) => {
        #[derive $derive]
        pub struct $name(pub $inner);

        relationship!(#[derive(Clone, Default, Debug, $crate::reexport::Reflect)] struct $name($inner) Deref @peer($peer));
        relationship!(#[derive(Clone, Default, Debug, $crate::reexport::Reflect)] struct $name($inner) Connectable @peer($peer));
    };
    (-- $type1:ident @peer($peer:ty)) => {
        relationship!(#[derive(Clone, Default, Debug, $crate::reexport::Reflect)] struct $type1($crate::RelationshipToOneEntity) @peer($peer));
    };
    (>- $type1:ident @peer($peer:ty)) => {
        relationship!(#[derive(Clone, Default, Debug, $crate::reexport::Reflect)] struct $type1($crate::RelationshipToManyEntity) @peer($peer));
    };
    (@Relationship $relationship:ident => $type1:ident) => {
        impl $crate::Peer for $type1{
            type Target = $type1;
        }

        #[derive(Default)]
        pub struct $relationship;
        impl $crate::Relationship for $relationship {
            type From = $type1;
            type To = $type1;
        }
    };
    (@Relationship $relationship:ident => $type1:ident - $type2:ident) => {
        impl $crate::Peer for $type1{
            type Target = $type2;
        }

        impl $crate::Peer for $type2{
            type Target = $type1;
        }

        #[derive(Default)]
        pub struct $relationship;
        impl $crate::Relationship for $relationship {
            type From = $type1;
            type To = $type2;
        }
    };
    (@Component: $type:ident $peer:ident) => {
        impl $crate::reexport::Component for $type{
            type Mutability = $crate::reexport::Mutable;
            const STORAGE_TYPE: $crate::reexport::StorageType= $crate::reexport::StorageType::Table;

            fn register_component_hooks(hooks: &mut $crate::reexport::ComponentHooks) {
                hooks.on_remove(|world, context|{
                    $crate::disconnect_all::<$type, $peer>(world, context.entity);
                });
            }
        }
    };
    (@Component own: $type:ident $peer:ident) => {
        impl $crate::reexport::Component for $type{
            type Mutability = $crate::reexport::Mutable;
            const STORAGE_TYPE: $crate::reexport::StorageType= $crate::reexport::StorageType::Table;

            fn register_component_hooks(hooks: &mut $crate::reexport::ComponentHooks) {
                hooks.on_remove(|world, context|{
                    $crate::disconnect_all_owned::<$type, $peer>(world, context.entity);
                });
            }
        }
    };
    ($relationship:ident => $type1:ident $(:$lifetime:ident)? -- $type2:ident) => {
        relationship!(-- $type1 @peer($type2));
        relationship!(-- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
        relationship!(@Component $($lifetime)?: $type1 $type2);
        relationship!(@Component: $type2 $type1);
    };
    ($relationship:ident => $type1:ident $(:$lifetime:ident)? -< $type2:ident) => {
        relationship!(>- $type1 @peer($type2));
        relationship!(-- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
        relationship!(@Component $($lifetime)?: $type1 $type2);
        relationship!(@Component: $type2 $type1);
    };
    ($relationship:ident => $type1:ident $(:$lifetime:ident)? >- $type2:ident) => {
        relationship!(-- $type1 @peer($type2));
        relationship!(>- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
        relationship!(@Component $($lifetime)?: $type1 $type2);
        relationship!(@Component: $type2 $type1);
    };
    ($relationship:ident => $type1:ident $(:$lifetime:ident)? >-< $type2:ident) => {
        relationship!(>- $type1 @peer($type2));
        relationship!(>- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
        relationship!(@Component $($lifetime)?: $type1 $type2);
        relationship!(@Component: $type2 $type1);
    };
    ($relationship:ident => @both -- $type1:ident) => {
        relationship!(-- $type1 @peer($type1));
        relationship!(@Relationship $relationship => $type1);
        relationship!(@Component: $type1 $type1);
    };
    ($relationship:ident => @both -< $type1:ident) => {
        relationship!(>- $type1 @peer($type1));
        relationship!(@Relationship $relationship => $type1);
        relationship!(@Component: $type1 $type1);
    };
}
