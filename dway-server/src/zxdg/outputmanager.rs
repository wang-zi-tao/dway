use crate::prelude::*;
use std::sync::Arc;

#[derive(Component)]
pub struct WlOutputManager {
    raw: zxdg_output_manager_v1::ZxdgOutputManagerV1,
}
#[derive(Component)]
pub struct ZxdgOutput {
    raw: zxdg_output_v1::ZxdgOutputV1,
}

#[derive(Resource)]
pub struct XdgOutputManagerDelegate(pub GlobalId);
delegate_dispatch!(DWay: [zxdg_output_manager_v1::ZxdgOutputManagerV1: Entity] => XdgOutputManagerDelegate);
impl
    wayland_server::Dispatch<
        zxdg_output_manager_v1::ZxdgOutputManagerV1,
        bevy::prelude::Entity,
        DWay,
    > for XdgOutputManagerDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &zxdg_output_manager_v1::ZxdgOutputManagerV1,
        request: <zxdg_output_manager_v1::ZxdgOutputManagerV1 as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &wayland_server::DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            zxdg_output_manager_v1::Request::Destroy => todo!(),
            zxdg_output_manager_v1::Request::GetXdgOutput { id, output } => {
                state.spawn_child_object(*data, id, data_init, |o| ZxdgOutput { raw: o });
            }
            _ => todo!(),
        }
    }
}
impl wayland_server::Dispatch<zxdg_output_v1::ZxdgOutputV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zxdg_output_v1::ZxdgOutputV1,
        request: <zxdg_output_v1::ZxdgOutputV1 as wayland_server::Resource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            <zxdg_output_v1::ZxdgOutputV1 as wayland_server::Resource>::Request::Destroy => {
                todo!()
            }
            _ => {
                todo!()
            }
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
impl GlobalDispatch<zxdg_output_manager_v1::ZxdgOutputManagerV1, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.init_object(resource, data_init, |o| WlOutputManager { raw: o });
    }
}

pub struct XdgOutputManagerPlugin(pub Arc<DisplayHandle>);
impl Plugin for XdgOutputManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(XdgOutputManagerDelegate(
            self.0
                .create_global::<DWay, zxdg_output_manager_v1::ZxdgOutputManagerV1, ()>(3, ()),
        ));
    }
}
