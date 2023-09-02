use crate::{
    geometry::Geometry,
    input::{
        grab::{Grab, ResizeEdges},
        seat::WlSeat,
    },
    prelude::*,
    resource::ResourceWrapper,
    wl::surface::WlSurface,
};

use super::DWayWindow;

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
    pub min_size: Option<IVec2>,
    pub max_size: Option<IVec2>,
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
            min_size: None,
            max_size: None,
            send_configure: false,
        }
    }
    pub fn resize(&self, size: IVec2) {
        debug!(
            "configure toplevel: {:?}",
            (size.x, size.y, vec![4, 0, 0, 0])
        );
        if self.raw.version() >= xdg_toplevel::EVT_CONFIGURE_BOUNDS_SINCE {
            self.raw.configure_bounds(size.x, size.y);
        }
        self.raw.configure(size.x, size.y, vec![4, 0, 0, 0]);
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
        _client: &wayland_server::Client,
        resource: &xdg_toplevel::XdgToplevel,
        request: <xdg_toplevel::XdgToplevel as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        match request {
            xdg_toplevel::Request::Destroy => {
                state.send_event(Destroy::<DWayWindow>::new(*data));
                state.destroy_object(resource);
            }
            xdg_toplevel::Request::SetParent { parent } => {
                let _parent_entity = parent.as_ref().map(DWay::get_entity);
                if let Some(parent) = &parent {
                    state.add_child(DWay::get_entity(parent), *data);
                }
            }
            xdg_toplevel::Request::SetTitle { title } => {
                state.with_component(resource, |c: &mut XdgToplevel| c.title = Some(title));
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                state.with_component(resource, |c: &mut XdgToplevel| c.app_id = Some(app_id));
            }
            xdg_toplevel::Request::ShowWindowMenu {
                seat: _,
                serial: _,
                x: _,
                y: _,
            } => {
                warn!("TODO: xdg_toplevel::Request::ShowWindowMenu");
            }
            xdg_toplevel::Request::Move { seat, serial } => {
                let rect = state.query::<&Geometry, _, _>(*data, |r| r.geometry);
                let pos = rect.pos();
                state.query::<(&mut Grab, &mut WlSeat), _, _>(
                    DWay::get_entity(&seat),
                    |(mut grab, mut seat)| {
                        *grab = Grab::Moving {
                            surface: *data,
                            serial,
                            relative: pos - seat.pointer_position.unwrap_or_default(),
                        };
                        seat.disable();
                    },
                );
            }
            xdg_toplevel::Request::Resize {
                seat,
                serial,
                edges,
            } => {
                let edges = match edges {
                    WEnum::Value(xdg_toplevel::ResizeEdge::Top) => ResizeEdges::TOP,
                    WEnum::Value(xdg_toplevel::ResizeEdge::TopRight) => {
                        ResizeEdges::TOP | ResizeEdges::RIGHT
                    }
                    WEnum::Value(xdg_toplevel::ResizeEdge::Right) => ResizeEdges::RIGHT,
                    WEnum::Value(xdg_toplevel::ResizeEdge::BottomRight) => {
                        ResizeEdges::BUTTOM | ResizeEdges::RIGHT
                    }
                    WEnum::Value(xdg_toplevel::ResizeEdge::Bottom) => ResizeEdges::BUTTOM,
                    WEnum::Value(xdg_toplevel::ResizeEdge::BottomLeft) => {
                        ResizeEdges::BUTTOM | ResizeEdges::LEFT
                    }
                    WEnum::Value(xdg_toplevel::ResizeEdge::Left) => ResizeEdges::LEFT,
                    WEnum::Value(xdg_toplevel::ResizeEdge::TopLeft) => {
                        ResizeEdges::TOP | ResizeEdges::LEFT
                    }
                    _ => return,
                };
                let (_surface, rect) = state
                    .query::<(&WlSurface, &Geometry), _, _>(*data, |(s, r)| {
                        (s.raw.clone(), r.geometry)
                    });
                state.query::<(&mut Grab, &mut WlSeat), _, _>(
                    DWay::get_entity(&seat),
                    |(mut grab, mut seat)| {
                        *grab = Grab::Resizing {
                            surface: *data,
                            serial,
                            edges,
                            relative: rect.pos() - seat.pointer_position.unwrap_or_default(),
                            origin_rect: rect,
                        };
                        seat.disable();
                    },
                );
            }
            xdg_toplevel::Request::SetMaxSize { width, height } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.max_size = Some(IVec2::new(width, height))
                });
            }
            xdg_toplevel::Request::SetMinSize { width, height } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.min_size = Some(IVec2::new(width, height))
                });
            }
            xdg_toplevel::Request::SetMaximized => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.max = true;
                });
            }
            xdg_toplevel::Request::UnsetMaximized => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.max = false;
                });
            }
            xdg_toplevel::Request::SetFullscreen { output: _ } => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.fullscreen = true;
                });
            }
            xdg_toplevel::Request::UnsetFullscreen => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.fullscreen = false;
                });
            }
            xdg_toplevel::Request::SetMinimized => {
                state.with_component(resource, |c: &mut XdgToplevel| {
                    c.min = true;
                });
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
        state.despawn_object(*data, resource);
        state.send_event(Destroy::<DWayWindow>::new(*data));
    }
}

pub struct XdgToplevelPlugin;
impl Plugin for XdgToplevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Insert<XdgToplevel>>();
        app.add_event::<Destroy<XdgToplevel>>();
        app.register_type::<XdgToplevel>();
    }
}
