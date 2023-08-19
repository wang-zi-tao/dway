use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    state::EntityFactory,
};


use super::{
    positioner::{XdgPositioner, XdgPositionerBundle},
    XdgDelegate, XdgSurface, XdgSurfaceBundle,
};

#[derive(Component)]
pub struct XdgWmBase {
    pub raw: xdg_wm_base::XdgWmBase,
}

impl XdgWmBase {
    pub fn new(raw: xdg_wm_base::XdgWmBase) -> Self {
        Self { raw }
    }
}

delegate_dispatch!(DWay: [xdg_wm_base::XdgWmBase: Entity] => XdgDelegate);
impl wayland_server::Dispatch<xdg_wm_base::XdgWmBase, bevy::prelude::Entity, DWay> for XdgDelegate {
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        _resource: &xdg_wm_base::XdgWmBase,
        request: <xdg_wm_base::XdgWmBase as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            xdg_wm_base::Request::Destroy => todo!(),
            xdg_wm_base::Request::CreatePositioner { id } => {
                state.spawn(
                    (id, data_init, |o| {
                        XdgPositionerBundle::new(XdgPositioner::new(o))
                    })
                        .with_parent(*data),
                );
            }
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                let entity = surface.data::<Entity>().unwrap();
                state.insert(
                    *entity,
                    (id, data_init, |o| XdgSurfaceBundle {
                        resource: XdgSurface::new(o),
                        geometry: Geometry::default(),
                        global_geometry: GlobalGeometry::default(),
                    }),
                );
                state.send_event(Insert::<XdgSurface>::new(*entity));
            }
            xdg_wm_base::Request::Pong { serial: _ } => todo!(),
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
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<xdg_wm_base::XdgWmBase>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| XdgWmBase { raw: o });
    }
}
