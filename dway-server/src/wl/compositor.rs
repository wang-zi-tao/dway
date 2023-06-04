use std::sync::Arc;

use wayland_server::delegate_global_dispatch;

use crate::{
    prelude::*,
    wl::surface::{WlSubsurface, WlSurface},
};

#[derive(Component)]
pub struct WlCompositor {
    raw: wl_compositor::WlCompositor,
}
#[derive(Component)]
pub struct WlSubcompositor {
    raw: wl_subcompositor::WlSubcompositor,
}

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
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                let parent = *data;
                let world = state.world_mut();
                let entity = world.spawn_empty().id();
                let object = data_init.init(id, entity);
                trace!(parent=?parent,?entity,object=?wayland_server::Resource::id(&object),"spawn object");
                let mut assets = world.resource_mut::<Assets<Image>>();
                let component = WlSurface::new(object, &mut assets);
                world.entity_mut(entity).insert(component);
                world.entity_mut(parent).add_child(entity);
            }
            wl_compositor::Request::CreateRegion { id } => {
                state.insert_object(*data, id, data_init, crate::wl::region::WlRegion::new);
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data,resource);
    }
}
delegate_dispatch!(DWay: [wl_subcompositor::WlSubcompositor: Entity] => CompositorDelegate);
impl wayland_server::Dispatch<wl_subcompositor::WlSubcompositor, bevy::prelude::Entity, DWay>
    for CompositorDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_subcompositor::WlSubcompositor,
        request: <wl_subcompositor::WlSubcompositor as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_subcompositor::Request::Destroy => todo!(),
            wl_subcompositor::Request::GetSubsurface {
                id,
                surface,
                parent,
            } => {
                state.insert_child_object(
                    DWay::get_entity(&parent),
                    DWay::get_entity(&surface),
                    id,
                    data_init,
                    WlSubsurface::new,
                );
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data,resource);
    }
}

delegate_global_dispatch!(DWay: [wl_compositor::WlCompositor: Entity] => CompositorDelegate);
impl wayland_server::GlobalDispatch<wl_compositor::WlCompositor, bevy::prelude::Entity, DWay>
    for CompositorDelegate
{
    fn bind(
        state: &mut DWay,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_compositor::WlCompositor>,
        global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        todo!()
    }
}

impl wayland_server::GlobalDispatch<wl_compositor::WlCompositor, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_compositor::WlCompositor>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.init_object(resource, data_init, |o| WlCompositor { raw: o });
    }
}
impl wayland_server::GlobalDispatch<wl_subcompositor::WlSubcompositor, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_subcompositor::WlSubcompositor>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.init_object(resource, data_init, |o| WlSubcompositor { raw: o });
    }
}

pub struct WlCompositorPlugin(pub Arc<DisplayHandle>);
impl Plugin for WlCompositorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CompositorDelegate {
            compositor: self
                .0
                .create_global::<DWay, wl_compositor::WlCompositor, ()>(5, ()),
            sub_compositor: self
                .0
                .create_global::<DWay, wl_subcompositor::WlSubcompositor, ()>(1, ()),
        });
    }
}
