pub mod popup;
pub mod toplevel;
pub mod wm;

use wayland_server::Resource;

use crate::{prelude::*, resource::ResourceWrapper, util::rect::IRect, xdg::toplevel::XdgToplevel, input::keyboard::WlKeyboard};
use std::sync::Arc;

#[derive(Resource)]
pub struct XdgDelegate {
    pub wm: GlobalId,
}
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgSurface {
    #[reflect(ignore)]
    pub raw: xdg_surface::XdgSurface,
    pub geometry: Option<IRect>,
}
impl ResourceWrapper for XdgSurface {
    type Resource = xdg_surface::XdgSurface;

    fn get_resource(&self) -> &Self::Resource {
        &self.raw
    }
}

impl XdgSurface {
    pub fn new(raw: xdg_surface::XdgSurface) -> Self {
        Self {
            raw,
            geometry: None,
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
                state.insert_object(*data, id, data_init, XdgToplevel::new);
                state.send_event(Insert::<XdgSurface>::new(*data));
            }
            xdg_surface::Request::GetPopup {
                id,
                parent,
                positioner,
            } => todo!(),
            xdg_surface::Request::SetWindowGeometry {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut XdgSurface| {
                    c.geometry = Some(IRect::from_pos_size(
                        IVec2::new(x, y),
                        IVec2::new(width, height),
                    ));
                });
            }
            xdg_surface::Request::AckConfigure { serial } => todo!(),
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

pub struct XdgShellPlugin(pub Arc<DisplayHandle>);
impl Plugin for XdgShellPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(XdgDelegate {
            wm: self
                .0
                .create_global::<DWay, xdg_wm_base::XdgWmBase, ()>(5, ()),
        });
        app.add_event::<Insert<XdgSurface>>();
        app.add_event::<Destroy<XdgSurface>>();
        app.register_type::<XdgSurface>();
    }
}
