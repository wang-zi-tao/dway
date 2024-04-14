use crate::{
    prelude::*,
    state::{add_global_dispatch, EntityFactory},
    wl::surface::{ClientHasSurface, WlSubsurface, WlSurface, WlSurfaceBundle},
};
use bevy_relationship::relationship;

#[derive(Component)]
pub struct WlCompositor {
    pub raw: wl_compositor::WlCompositor,
}
#[derive(Component)]
pub struct WlSubcompositor {
    pub raw: wl_subcompositor::WlSubcompositor,
}
relationship!(HasSubsurface=>SubsurfaceList-<ParentSurface);

#[derive(Resource)]
pub struct CompositorDelegate {
    pub compositor: GlobalId,
    pub sub_compositor: GlobalId,
}
delegate_dispatch!(DWay: [wl_compositor::WlCompositor: Entity] => CompositorDelegate);
impl wayland_server::Dispatch<wl_compositor::WlCompositor, bevy::prelude::Entity, DWay>
    for CompositorDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_compositor::WlCompositor,
        request: <wl_compositor::WlCompositor as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                let parent = *data;
                let world = state.world_mut();
                let entity_mut = world.spawn_empty();
                let entity = entity_mut.id();
                let object = data_init.init(id, entity);
                trace!(parent=?parent,?entity,object=?wayland_server::Resource::id(&object),"spawn object");
                let mut assets = world.resource_mut::<Assets<Image>>();
                let component = WlSurface::new(object, &mut assets);
                world
                    .entity_mut(entity)
                    .insert(WlSurfaceBundle::new(component));
                world.entity_mut(parent).add_child(entity);
                state.connect::<ClientHasSurface>(DWay::client_entity(client), entity);
            }
            wl_compositor::Request::CreateRegion { id } => {
                state.spawn_child_object(*data, id, data_init, crate::wl::region::WlRegion::new);
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_compositor::WlCompositor,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<WlCompositor>(*data, resource);
    }
}
delegate_dispatch!(DWay: [wl_subcompositor::WlSubcompositor: Entity] => CompositorDelegate);
impl wayland_server::Dispatch<wl_subcompositor::WlSubcompositor, bevy::prelude::Entity, DWay>
    for CompositorDelegate
{
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wl_subcompositor::WlSubcompositor,
        request: <wl_subcompositor::WlSubcompositor as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_subcompositor::Request::Destroy => todo!(),
            wl_subcompositor::Request::GetSubsurface {
                id,
                surface,
                parent,
            } => {
                let Some(entity) = state
                    .insert(
                        DWay::get_entity(&parent),
                        (id, data_init, WlSubsurface::new).with_parent(DWay::get_entity(&surface)),
                    )
                    .map(|e| e.id())
                else {
                    return;
                };
                state.connect::<HasSubsurface>(DWay::get_entity(&parent), entity);
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_subcompositor::WlSubcompositor,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<WlSubcompositor>(*data, resource);
    }
}

impl wayland_server::GlobalDispatch<wl_compositor::WlCompositor, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_compositor::WlCompositor>,
        _global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| WlCompositor { raw: o });
    }
}
impl wayland_server::GlobalDispatch<wl_subcompositor::WlSubcompositor, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_subcompositor::WlSubcompositor>,
        _global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| WlSubcompositor { raw: o });
    }
}

pub struct WlCompositorPlugin;
impl Plugin for WlCompositorPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wl_compositor::WlCompositor, 6>(app);
        add_global_dispatch::<wl_subcompositor::WlSubcompositor, 1>(app);
        app.register_relation::<HasSubsurface>();
    }
}
