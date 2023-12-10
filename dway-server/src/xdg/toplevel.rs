use crate::{
    geometry::Geometry,
    input::grab::{ResizeEdges, SurfaceGrabKind, WlSurfacePointerState},
    prelude::*,
    resource::ResourceWrapper,
    schedule::DWayServerSet,
};

use super::{DWayWindow, XdgSurface};

#[derive(Component)]
pub struct PinedWindow;

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Debug)]
pub struct DWayToplevel {
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub max: bool,
    pub fullscreen: bool,
    pub min: bool,
    pub min_size: Option<IVec2>,
    pub max_size: Option<IVec2>,
    pub size: Option<IVec2>,
}

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgToplevel {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_toplevel::XdgToplevel,
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
            send_configure: false,
        }
    }
    pub fn configure(&self, data: &DWayToplevel) {
        let mut states = vec![];

        states.extend((xdg_toplevel::State::Activated as u32).to_le_bytes());
        if data.max {
            states.extend((xdg_toplevel::State::Maximized as u32).to_le_bytes());
        }
        if data.fullscreen {
            states.extend((xdg_toplevel::State::Fullscreen as u32).to_le_bytes());
        }

        if self.raw.version() >= xdg_toplevel::EVT_CONFIGURE_BOUNDS_SINCE {
            self.raw.configure_bounds(
                data.size.unwrap_or_default().x,
                data.size.unwrap_or_default().y,
            );
        }

        self.raw.configure(
            data.size.unwrap_or_default().x,
            data.size.unwrap_or_default().y,
            states,
        );
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
        debug!("request {:?}", &request);
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
                state.with_component(resource, |c: &mut DWayToplevel| c.title = Some(title));
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                state.with_component(resource, |c: &mut DWayToplevel| c.app_id = Some(app_id));
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
                if state.entity(*data).contains::<PinedWindow>() {
                    return;
                }
                if let Some(mut pointer_state) = state.get_mut::<WlSurfacePointerState>(*data) {
                    pointer_state.grab = Some(Box::new(SurfaceGrabKind::Move {
                        seat: DWay::get_entity(&seat),
                        serial,
                    }));
                }
            }
            xdg_toplevel::Request::Resize {
                seat,
                serial,
                edges,
            } => {
                if state.entity(*data).contains::<PinedWindow>() {
                    return;
                }
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
                if let Some(mut pointer_state) = state.get_mut::<WlSurfacePointerState>(*data) {
                    pointer_state.grab = Some(Box::new(SurfaceGrabKind::Resizing {
                        seat: DWay::get_entity(&seat),
                        serial,
                        edges,
                    }));
                }
            }
            xdg_toplevel::Request::SetMaxSize { width, height } => {
                if let Some(mut c) = state.get_mut::<DWayToplevel>(*data) {
                    if c.max_size != Some(IVec2::new(width, height)) {
                        c.max_size = Some(IVec2::new(width, height))
                    }
                }
            }
            xdg_toplevel::Request::SetMinSize { width, height } => {
                if let Some(mut c) = state.get_mut::<DWayToplevel>(*data) {
                    if c.min_size != Some(IVec2::new(width, height)) {
                        c.min_size = Some(IVec2::new(width, height))
                    }
                }
            }
            xdg_toplevel::Request::SetMaximized => {
                state.with_component(resource, |c: &mut DWayToplevel| {
                    c.max = true;
                });
            }
            xdg_toplevel::Request::UnsetMaximized => {
                state.with_component(resource, |c: &mut DWayToplevel| {
                    c.max = false;
                });
            }
            xdg_toplevel::Request::SetFullscreen { output: _ } => {
                state.with_component(resource, |c: &mut DWayToplevel| {
                    c.fullscreen = true;
                });
            }
            xdg_toplevel::Request::UnsetFullscreen => {
                state.with_component(resource, |c: &mut DWayToplevel| {
                    c.fullscreen = false;
                });
            }
            xdg_toplevel::Request::SetMinimized => {
                state.with_component(resource, |c: &mut DWayToplevel| {
                    c.min = true;
                });
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &xdg_toplevel::XdgToplevel,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
        state.send_event(Destroy::<DWayWindow>::new(*data));
    }
}

pub fn process_window_action_event(
    mut events: EventReader<WindowAction>,
    mut window_query: Query<
        (
            &mut XdgToplevel,
            &mut DWayToplevel,
            &XdgSurface,
            &mut Geometry,
        ),
        With<DWayWindow>,
    >,
) {
    for e in events.read() {
        match e {
            WindowAction::Close(e) => {
                if let Ok((c, _t, s, _)) = window_query.get_mut(*e) {
                    c.raw.close();
                    s.configure();
                }
            }
            WindowAction::Maximize(e) => {
                if let Ok((c, mut t, s, _)) = window_query.get_mut(*e) {
                    t.max = true;
                    c.configure(&mut t);
                    s.configure();
                }
            }
            WindowAction::UnMaximize(e) => {
                if let Ok((c, mut t, s, _)) = window_query.get_mut(*e) {
                    t.max = true;
                    c.configure(&mut t);
                    s.configure();
                }
            }
            WindowAction::Fullscreen(e) => {
                if let Ok((c, mut t, s, _)) = window_query.get_mut(*e) {
                    t.fullscreen = true;
                    c.configure(&mut t);
                    s.configure();
                }
            }
            WindowAction::UnFullscreen(e) => {
                if let Ok((c, mut t, s, _)) = window_query.get_mut(*e) {
                    t.fullscreen = false;
                    c.configure(&mut t);
                    s.configure();
                }
            }
            WindowAction::Minimize(_) => {}
            WindowAction::UnMinimize(_) => {}
            WindowAction::SetRect(e, rect) => {
                if let Ok((c, mut t, s, mut g)) = window_query.get_mut(*e) {
                    g.geometry.set_pos(rect.pos());
                    t.size = Some(rect.size());
                    c.configure(&mut t);
                    s.configure();
                }
            }
        }
    }
}

pub struct XdgToplevelPlugin;
impl Plugin for XdgToplevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Insert<XdgToplevel>>();
        app.add_event::<Destroy<XdgToplevel>>();
        app.register_type::<XdgToplevel>();
        app.add_systems(
            Last,
            process_window_action_event.in_set(DWayServerSet::ProcessWindowAction),
        );
    }
}
