use crate::{geometry::GlobalGeometry, prelude::*, xdg::toplevel::XdgToplevel};
use std::sync::Arc;

use super::{XdgDelegate, XdgSurface};

#[derive(Component)]
pub struct XdgWmBase {
    raw: xdg_wm_base::XdgWmBase,
}

delegate_dispatch!(DWay: [xdg_wm_base::XdgWmBase: Entity] => XdgDelegate);
impl wayland_server::Dispatch<xdg_wm_base::XdgWmBase, bevy::prelude::Entity, DWay> for XdgDelegate {
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &xdg_wm_base::XdgWmBase,
        request: <xdg_wm_base::XdgWmBase as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            xdg_wm_base::Request::Destroy => todo!(),
            xdg_wm_base::Request::CreatePositioner { id } => todo!(),
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                let entity = surface.data::<Entity>().unwrap();
                state.insert_object_bundle(*entity, id, data_init, |o| {
                    (XdgSurface::new(o), (GlobalGeometry::default()))
                });
                state.send_event(Insert::<XdgSurface>::new(*entity));
            }
            xdg_wm_base::Request::Pong { serial } => todo!(),
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        _resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn(*data);
    }
}
impl wayland_server::GlobalDispatch<xdg_wm_base::XdgWmBase, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<xdg_wm_base::XdgWmBase>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.init_object(resource, data_init, |o| XdgWmBase { raw: o });
    }
}
