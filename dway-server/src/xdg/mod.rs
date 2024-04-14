pub mod activation;
pub mod activation_token;
pub mod popup;
pub mod positioner;
pub mod toplevel;
pub mod wm;

use self::{
    activation::{SurfaceActivate, XdgActivation},
    activation_token::XdgActivationToken,
    wm::XdgWmBase,
};
use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::grab::WlSurfacePointerState,
    prelude::*,
    resource::ResourceWrapper,
    state::{add_global_dispatch, EntityFactory},
    util::{rect::IRect, serial::next_serial},
    wl::surface::WlSurface,
    xdg::{
        popup::{XdgPopup, XdgPopupBundle},
        positioner::XdgPositioner,
        toplevel::{DWayToplevel, XdgToplevel},
    },
};
use bevy_relationship::relationship;

#[derive(Component, Default, Clone, Reflect)]
pub struct DWayWindow {}

#[derive(Resource)]
pub struct XdgDelegate {
    pub wm: GlobalId,
}
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgSurface {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_surface::XdgSurface,
    pub send_configure: bool,
}
impl ResourceWrapper for XdgSurface {
    type Resource = xdg_surface::XdgSurface;

    fn get_resource(&self) -> &Self::Resource {
        &self.raw
    }
}
#[derive(Bundle)]
pub struct XdgSurfaceBundle {
    resource: XdgSurface,
    geometry: Geometry,
    global_geometry: GlobalGeometry,
    seat_state: WlSurfacePointerState,
}

impl XdgSurface {
    pub fn new(raw: xdg_surface::XdgSurface) -> Self {
        Self {
            raw,
            send_configure: false,
        }
    }
    pub fn configure(&self) {
        self.raw.configure(next_serial());
    }
}
relationship!(SurfaceHasPopup=>PopupList-<PopupParent);
delegate_dispatch!(DWay: [xdg_surface::XdgSurface: Entity] => XdgDelegate);
impl wayland_server::Dispatch<xdg_surface::XdgSurface, bevy::prelude::Entity, DWay>
    for XdgDelegate
{
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &xdg_surface::XdgSurface,
        request: <xdg_surface::XdgSurface as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_surface::Request::Destroy => { }
            xdg_surface::Request::GetToplevel { id } => {
                state.insert(
                    *data,
                    (
                        id,
                        data_init,
                        (|o| {
                            (
                                XdgToplevel::new(o),
                                DWayToplevel::default(),
                                DWayWindow::default(),
                            )
                        }),
                    )
                        .check_component_not_exists::<XdgToplevel>(),
                );
                state.send_event(Insert::<DWayWindow>::new(*data));
                state.query_object::<(&mut XdgSurface, &mut XdgToplevel), _, _>(
                    resource,
                    |(mut xdg_surface, mut xdg_toplevel)| {
                        if !xdg_toplevel.send_configure {
                            debug!("toplevel send configure ({},{})", 800, 600);
                            xdg_toplevel.raw.configure(0, 0, vec![4, 0, 0, 0]);
                            xdg_toplevel.send_configure = true;
                        }
                        if !xdg_surface.send_configure {
                            debug!("xdg_surface send configure");
                            xdg_surface.raw.configure(next_serial());
                            xdg_surface.send_configure = true;
                        }
                    },
                );
            }
            xdg_surface::Request::GetPopup {
                id,
                parent,
                positioner,
            } => {
                let level = state
                    .get::<XdgPopup>(*data)
                    .map(|popup| popup.level + 1)
                    .unwrap_or_default();
                let positioner = state
                    .query_object_component(&positioner, |c: &mut XdgPositioner| {
                        c.positioner.clone()
                    });
                let parent_entity = parent.map(|r| DWay::get_entity(&r)).unwrap_or(*data);
                let geometry = positioner.get_geometry();
                state.insert(
                    *data,
                    (id, data_init, |o| XdgPopupBundle {
                        raw: XdgPopup::new(o, positioner.clone(), level),
                        geometry: Geometry::new(geometry),
                        global_geometry: GlobalGeometry::new(geometry),
                        window: Default::default(),
                    })
                        .with_parent(parent_entity),
                );
                state.send_event(Insert::<DWayWindow>::new(*data));
                state.connect::<SurfaceHasPopup>(parent_entity, *data);
                state.query_object::<(&mut XdgSurface, &mut XdgPopup), _, _>(
                    resource,
                    |(mut xdg_surface, mut popup)| {
                        if !popup.send_configure {
                            debug!("popup send configure {:?}", geometry);
                            popup.raw.configure(
                                geometry.x(),
                                geometry.y(),
                                geometry.width(),
                                geometry.height(),
                            );
                            popup.send_configure = true;
                        }
                        if !xdg_surface.send_configure {
                            debug!("xdg_surface send configure");
                            xdg_surface.raw.configure(next_serial());
                            xdg_surface.send_configure = true;
                        }
                    },
                );
            }
            xdg_surface::Request::SetWindowGeometry {
                x,
                y,
                width,
                height,
            } => {
                if let Some(mut c) = state.get_mut::<WlSurface>(*data) {
                    let rect = IRect::from_pos_size(IVec2::new(x, y), IVec2::new(width, height));
                    if c.pending.window_geometry != Some(rect) {
                        c.pending.window_geometry = Some(rect);
                    }
                }
            }
            xdg_surface::Request::AckConfigure { serial } => {
                debug!("popup AckConfigure {serial}");
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &xdg_surface::XdgSurface,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<XdgSurface>(*data, resource);
    }
}
impl
    wayland_server::GlobalDispatch<
        wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase,
        bevy::prelude::Entity,
    > for DWay
{
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<
            wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase,
        >,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, XdgWmBase::new);
    }
}

pub struct XdgShellPlugin;
impl Plugin for XdgShellPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<xdg_wm_base::XdgWmBase, 6>(app);
        add_global_dispatch::<xdg_activation_v1::XdgActivationV1, 1>(app);
        app.register_relation::<SurfaceHasPopup>();
        app.add_event::<Insert<DWayWindow>>();
        app.add_event::<Destroy<DWayWindow>>();
        app.register_type::<DWayWindow>();
        app.register_type::<XdgSurface>();
        app.register_type::<XdgActivationToken>();
        app.register_type::<XdgActivation>();
        app.register_type::<SurfaceActivate>();
    }
}
