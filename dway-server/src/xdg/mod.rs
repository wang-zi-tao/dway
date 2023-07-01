pub mod activation_token;
pub mod popup;
pub mod positioner;
pub mod toplevel;
pub mod wm;

use wayland_protocols::xdg::activation::v1::server::xdg_activation_token_v1;
use wayland_server::Resource;

use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::keyboard::WlKeyboard,
    prelude::*,
    resource::ResourceWrapper,
    state::create_global_system_config,
    util::{rect::IRect, serial::next_serial},
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, positioner::XdgPositioner, toplevel::XdgToplevel},
};
use std::sync::Arc;

use self::wm::XdgWmBase;

#[derive(Resource)]
pub struct XdgDelegate {
    pub wm: GlobalId,
}
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgSurface {
    #[reflect(ignore)]
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
}

impl XdgSurface {
    pub fn new(raw: xdg_surface::XdgSurface) -> Self {
        Self {
            raw,
            send_configure: false,
        }
    }
}
delegate_dispatch!(DWay: [xdg_surface::XdgSurface: Entity] => XdgDelegate);
impl wayland_server::Dispatch<xdg_surface::XdgSurface, bevy::prelude::Entity, DWay>
    for XdgDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &xdg_surface::XdgSurface,
        request: <xdg_surface::XdgSurface as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_surface::Request::Destroy => todo!(),
            xdg_surface::Request::GetToplevel { id } => {
                state.insert_object(*data, id, data_init, |o| XdgToplevel::new(o));
                state.send_event(Insert::<XdgSurface>::new(*data));
                state.query_object::<(&Geometry, &mut XdgSurface, &mut XdgToplevel), _, _>(
                    resource,
                    |(geometry, mut xdg_surface, mut xdg_toplevel)| {
                        let size = geometry.geometry.size();
                        if !xdg_toplevel.send_configure {
                            let size = geometry.geometry.size();
                            debug!("toplevel send configure ({},{})", 800, 800);
                            xdg_toplevel.raw.configure(800, 600, vec![4, 0, 0, 0]);
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
                let XdgPositioner {
                    raw: _,
                    anchor_rect,
                    constraint_adjustment,
                    anchor_kind,
                    gravity,
                    is_relative,
                } = state.query_object_component(&positioner, |c: &mut XdgPositioner| c.clone());
                let parent_entity = parent.map(|r| DWay::get_entity(&r)).unwrap_or(*data);
                state.spawn_child_object(parent_entity, id, data_init, |o| {
                    XdgPopup::new(
                        o,
                        anchor_rect,
                        constraint_adjustment,
                        anchor_kind,
                        gravity,
                        is_relative,
                    )
                });
            }
            xdg_surface::Request::SetWindowGeometry {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut WlSurface| {
                    c.pending.window_geometry = Some(IRect::from_pos_size(
                        IVec2::new(x, y),
                        IVec2::new(width, height),
                    ));
                });
            }
            xdg_surface::Request::AckConfigure { serial } => {}
            _ => todo!(),
        }
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
impl
    wayland_server::GlobalDispatch<
        wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase,
        bevy::prelude::Entity,
    > for DWay
{
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<
            wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase,
        >,
        global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, XdgWmBase::new);
    }
}

pub struct XdgShellPlugin;
impl Plugin for XdgShellPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(create_global_system_config::<xdg_wm_base::XdgWmBase, 5>());
        app.add_system(create_global_system_config::<
            xdg_activation_token_v1::XdgActivationTokenV1,
            1,
        >());
        app.add_event::<Insert<XdgSurface>>();
        app.add_event::<Destroy<XdgSurface>>();
        app.register_type::<XdgSurface>();
    }
}
