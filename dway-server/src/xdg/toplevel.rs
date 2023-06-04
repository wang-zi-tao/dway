use bevy::utils::tracing::Span;
use inlinable_string::{InlinableString, InlineString};

use crate::{prelude::*, resource::ResourceWrapper};
use std::sync::Arc;

use super::XdgSurface;

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgToplevel {
    #[reflect(ignore)]
    pub raw: xdg_toplevel::XdgToplevel,
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub max: bool,
    pub fullscreen: bool,
    pub min: bool,
    pub minSize: Option<IVec2>,
    pub maxSize: Option<IVec2>,
    pub send_configure: bool,
}
impl ResourceWrapper for XdgToplevel {
    type Resource = xdg_toplevel::XdgToplevel;

    fn get_resource(&self) -> &Self::Resource {
        &self.raw
    }
}
impl XdgToplevel {
    pub fn new(object: xdg_toplevel::XdgToplevel) -> Self {
        Self {
            raw: object,
            title: None,
            app_id: None,
            max: false,
            fullscreen: false,
            min: false,
            minSize: None,
            maxSize: None,
            send_configure: false,
        }
    }
}

#[derive(Resource)]
pub struct ToplevelDelegate(pub GlobalId);
delegate_dispatch!(DWay: [xdg_toplevel::XdgToplevel: Entity] => ToplevelDelegate);
impl wayland_server::Dispatch<xdg_toplevel::XdgToplevel, bevy::prelude::Entity, DWay>
    for ToplevelDelegate
{
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &xdg_toplevel::XdgToplevel,
        request: <xdg_toplevel::XdgToplevel as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        match request {
            xdg_toplevel::Request::Destroy => todo!(),
            xdg_toplevel::Request::SetParent { parent } => {
                let parent_entity = parent.as_ref().map(|p| DWay::get_entity(p));
                if parent.is_some() {
                    todo!();
                }
            }
            xdg_toplevel::Request::SetTitle { title } => {
                state.with_component(resource, |c: &mut XdgToplevel| c.title = Some(title.into()));
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.app_id = Some(app_id.into())
                });
            }
            xdg_toplevel::Request::ShowWindowMenu { seat, serial, x, y } => todo!(),
            xdg_toplevel::Request::Move { seat, serial } => {
                warn!("TODO: xdg_toplevel::Request::Move")
            }
            xdg_toplevel::Request::Resize {
                seat,
                serial,
                edges,
            } => {
                if let WEnum::Value(edges) = edges {}
                warn!("TODO: xdg_toplevel::Request::Resize")
            }
            xdg_toplevel::Request::SetMaxSize { width, height } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.maxSize = Some(IVec2::new(width, height))
                });
            }
            xdg_toplevel::Request::SetMinSize { width, height } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.minSize = Some(IVec2::new(width, height))
                });
            }
            xdg_toplevel::Request::SetMaximized => todo!(),
            xdg_toplevel::Request::UnsetMaximized => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.max = false;
                });
            }
            xdg_toplevel::Request::SetFullscreen { output } => todo!(),
            xdg_toplevel::Request::UnsetFullscreen => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.fullscreen = false;
                });
            }
            xdg_toplevel::Request::SetMinimized => todo!(),
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
        state.send_event(Destroy::<XdgSurface>::new(*data));
    }
}

pub struct XdgToplevelPlugin(pub Arc<DisplayHandle>);
impl Plugin for XdgToplevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Insert<XdgToplevel>>();
        app.add_event::<Destroy<XdgToplevel>>();
        app.register_type::<XdgToplevel>();
    }
}
