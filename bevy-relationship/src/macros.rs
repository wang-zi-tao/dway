#[macro_export]
macro_rules! relationship {
    (#[derive $derive:tt] struct $name:ident($inner:ty) @peer($peer:ty)) => {
        #[derive $derive]
        pub struct $name(pub $inner);

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
        impl $crate::Connectable for $name {
            type Iterator<'l> = <$inner as $crate::Connectable>::Iterator<'l>;

            fn iter<'l>(&'l self) -> Self::Iterator<'l> {
                self.0.iter()
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

            fn drain<'l>(&'l mut self) -> Self::Drain<'l> {
                self.0.drain()
            }

            fn get_sender_mut(&mut self)->&mut $crate::ConnectionEventSender {
                self.0.get_sender_mut()
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                for peer_entity in $crate::Connectable::iter(self) {
                    self.sender.send::<$peer>(peer_entity);
                }
            }
        }
    };
    (-- $type1:ident @peer($peer:ty)) => {
        relationship!(#[derive($crate::reexport::Component, Clone, Default, Debug, $crate::reexport::Reflect)] struct $type1($crate::RelationshipToOneEntity) @peer($peer));
    };
    (>- $type1:ident @peer($peer:ty)) => {
        relationship!(#[derive($crate::reexport::Component, Clone, Default, Debug, $crate::reexport::Reflect)] struct $type1($crate::RelationshipToManyEntity) @peer($peer));
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
    ($relationship:ident => $type1:ident -- $type2:ident) => {
        relationship!(-- $type1 @peer($type2));
        relationship!(-- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
    };
    ($relationship:ident => $type1:ident -< $type2:ident) => {
        relationship!(>- $type1 @peer($type2));
        relationship!(-- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
    };
    ($relationship:ident => $type1:ident >- $type2:ident) => {
        relationship!(-- $type1 @peer($type2));
        relationship!(>- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
    };
    ($relationship:ident => $type1:ident >-< $type2:ident) => {
        relationship!(>- $type1 @peer($type2));
        relationship!(>- $type2 @peer($type1));
        relationship!(@Relationship $relationship => $type1 - $type2);
    };
    ($relationship:ident => @both -- $type1:ident) => {
        relationship!(-- $type1 @peer($type1));
        relationship!(@Relationship $relationship => $type1);
    };
    ($relationship:ident => @both -< $type1:ident) => {
        relationship!(>- $type1 @peer($type1));
        relationship!(@Relationship $relationship => $type1);
    };
}
