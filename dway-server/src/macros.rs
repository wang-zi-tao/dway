pub use crate::prelude::*;

#[macro_export]
macro_rules! create_dispatch {
    (@global $resource:ty:$version:literal=>$name:ident) => {
        create_dispatch!($resource=>$name)
        impl GlobalDispatch< $resource, Entity, > for DWay {
            fn bind(
                state: &mut Self,
                handle: &DisplayHandle,
                client: &wayland_server::Client,
                resource: wayland_server::New<$resource>,
                global_data: &bevy::prelude::Entity,
                data_init: &mut wayland_server::DataInit<'_, Self>,
            ) {
                state.bind(client, resource, data_init, |o| {
                    $name::new(o)
               });
            }
        }
    };
    ($resource:ty=>$name:ident) => {
        #[derive(Component,Reflect,Debug)]
        #[reflect(Debug)]
        pub struct $name {
            #[reflect(ignore)]
            pub raw: $resource,
        }

        impl $name {
            pub fn new(raw: $resource) -> Self {
                Self {
                    raw,
                }
            }
        }
        
        impl Drop for $name {
            fn drop(&mut self) {
                trace!(entity=?DWay::get_entity(&self.raw), resource=?self.raw.id(), "drop wayland resource");
            }
        }

        impl Dispatch<$resource, Entity> for DWay {
            fn request(
                state: &mut Self,
                client: &wayland_server::Client,
                resource: &$resource,
                request: <$resource as WlResource>::Request,
                data: &Entity,
                dhandle: &DisplayHandle,
                data_init: &mut wayland_server::DataInit<'_, Self>,
            ) {
                let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
                let _enter = span.enter();
                debug!("request {:?}", &request);
                todo!();
            }

            fn destroyed(
                state: &mut DWay,
                _client: wayland_backend::server::ClientId,
                resource: wayland_backend::server::ObjectId,
                data: &bevy::prelude::Entity,
            ) {
                state.despawn_object(*data, resource);
            }
        }
    }
}
