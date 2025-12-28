use bevy::ecs::query::QueryData;
use smart_default::SmartDefault;

use super::{DWayWindow, XdgSurface};
use crate::{
    events::Insert, geometry::{Geometry, GlobalGeometry, set_geometry}, input::{
        grab::{ResizeEdges, StartGrab, WlSurfacePointerState},
        seat::WlSeat,
    }, prelude::*, resource::ResourceWrapper, wl::surface::{ClientHasSurface, WlSurface}
};

#[derive(Component)]
pub struct PinedWindow;

#[derive(Component, Reflect, Debug, Clone, SmartDefault)]
#[reflect(Debug)]
pub struct DWayToplevel {
    pub title: Option<String>,
    pub app_id: Option<String>,
    pub max: bool,
    pub fullscreen: bool,
    pub min: bool,
    pub decorated: bool,
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

    pub fn configure(&self, surface: &WlSurface, data: &DWayToplevel) {
        let mut states = vec![];

        states.extend((xdg_toplevel::State::Activated as u32).to_le_bytes());
        if data.max {
            states.extend((xdg_toplevel::State::Maximized as u32).to_le_bytes());
        }
        if data.fullscreen {
            states.extend((xdg_toplevel::State::Fullscreen as u32).to_le_bytes());
        }

        let size = surface.calculate_toplevel_size(data.size.unwrap_or_default());

        if self.raw.version() >= xdg_toplevel::EVT_CONFIGURE_BOUNDS_SINCE {
            self.raw.configure_bounds(size.x, size.y);
        }

        self.raw.configure(size.x, size.y, states);
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
                state.entity_mut(*data).insert(Name::new(format!(
                    "{:?} {:?}",
                    resource.id(),
                    title
                )));
                state.with_component_mut(resource, |c: &mut DWayToplevel| c.title = Some(title));
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
                    c.app_id = Some(app_id.clone())
                });
                state.send_event(WindowAppIdChanged {
                    entity: *data,
                    app_id,
                });
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
                let mouse_pos = state
                    .get_mut::<WlSurfacePointerState>(*data)
                    .unwrap()
                    .mouse_pos;
                let geometry = state.get_mut::<Geometry>(*data).unwrap().clone();

                state.send_event(StartGrab::Move {
                    mouse_pos,
                    seat: DWay::get_entity(&seat),
                    serial: Some(serial),
                    surface: *data,
                    geometry,
                });
            }
            xdg_toplevel::Request::Resize {
                seat,
                serial: _,
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
                let geometry = state.get_mut::<Geometry>(*data).unwrap().clone();
                state.send_event(StartGrab::Resizing {
                    seat: DWay::get_entity(&seat),
                    serial: None,
                    edges,
                    geometry,
                    surface: *data,
                });
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
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
                    c.max = true;
                });
            }
            xdg_toplevel::Request::UnsetMaximized => {
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
                    c.max = false;
                });
            }
            xdg_toplevel::Request::SetFullscreen { output: _ } => {
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
                    c.fullscreen = true;
                });
            }
            xdg_toplevel::Request::UnsetFullscreen => {
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
                    c.fullscreen = false;
                });
            }
            xdg_toplevel::Request::SetMinimized => {
                state.with_component_mut(resource, |c: &mut DWayToplevel| {
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

graph_query!(InputGraph=>[
    surface=Entity,
    client=(Entity, &'static mut WlSeat ),
]=>{
    pointer=surface<-[ClientHasSurface]-client
});

#[derive(QueryData)]
#[query_data(mutable)]
pub struct ToplevelWorldQuery {
    xdg_obj: &'static mut XdgToplevel,
    data: &'static mut DWayToplevel,
    xdg_surface: &'static XdgSurface,
    wl_surface: &'static WlSurface,
    geo: &'static mut Geometry,
    global_geo: &'static mut GlobalGeometry,
    pinned: Option<&'static PinedWindow>,
    pointer_state: &'static mut WlSurfacePointerState,
}

pub fn update_window(
    mut windows: Query<(
        &XdgToplevel,
        &XdgSurface,
        &WlSurface,
        &mut DWayToplevel,
        Ref<Geometry>,
    )>,
) {
    for (toplevel, xdg_surface, wl_surface, mut data, geometry) in &mut windows {
        if geometry.is_changed() && Some(geometry.size()) != data.size {
            data.size = Some(geometry.size());
            toplevel.configure(wl_surface, &data);
            xdg_surface.configure();
        }
    }
}

pub fn receive_window_action_event(
    mut graph: InputGraph,
    mut events: MessageReader<WindowAction>,
    mut window_query: Query<ToplevelWorldQuery, With<DWayWindow>>,
    mut start_grab_events: MessageWriter<StartGrab>,
) {
    for e in events.read() {
        match e {
            WindowAction::Close(e) => {
                if let Ok(toplevel) = window_query.get_mut(*e) {
                    toplevel.xdg_obj.raw.close();
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::Maximize(e) => {
                if let Ok(mut toplevel) = window_query.get_mut(*e) {
                    toplevel.data.max = true;
                    toplevel
                        .xdg_obj
                        .configure(toplevel.wl_surface, &toplevel.data);
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::UnMaximize(e) => {
                if let Ok(mut toplevel) = window_query.get_mut(*e) {
                    toplevel.data.max = true;
                    toplevel
                        .xdg_obj
                        .configure(toplevel.wl_surface, &toplevel.data);
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::Fullscreen(e) => {
                if let Ok(mut toplevel) = window_query.get_mut(*e) {
                    toplevel.data.fullscreen = true;
                    toplevel
                        .xdg_obj
                        .configure(toplevel.wl_surface, &toplevel.data);
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::UnFullscreen(e) => {
                if let Ok(mut toplevel) = window_query.get_mut(*e) {
                    toplevel.data.fullscreen = false;
                    toplevel
                        .xdg_obj
                        .configure(toplevel.wl_surface, &toplevel.data);
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::Minimize(_) => {}
            WindowAction::UnMinimize(_) => {}
            WindowAction::SetRect(e, rect) => {
                if let Ok(mut toplevel) = window_query.get_mut(*e) {
                    set_geometry(&mut toplevel.geo, &mut toplevel.global_geo, *rect);
                    toplevel.data.size = Some(rect.size());
                    toplevel
                        .xdg_obj
                        .configure(toplevel.wl_surface, &toplevel.data);
                    toplevel.xdg_surface.configure();
                }
            }
            WindowAction::RequestMove(e) => {
                if let Ok(toplevel) = window_query.get_mut(*e) {
                    if toplevel.pinned.is_some() {
                        return;
                    }
                    graph.for_each_pointer_mut_from(*e, |_, (seat_entity, _)| {
                        start_grab_events.write(StartGrab::Move {
                            surface: *e,
                            seat: *seat_entity,
                            serial: None,
                            mouse_pos: toplevel.pointer_state.mouse_pos,
                            geometry: toplevel.geo.clone(),
                        });
                        ControlFlow::<()>::default()
                    });
                }
            }
            WindowAction::RequestResize(e, edges) => {
                if let Ok(toplevel) = window_query.get_mut(*e) {
                    if toplevel.pinned.is_some() {
                        return;
                    }
                    graph.for_each_pointer_mut_from(*e, |_, (seat_entity, _)| {
                        debug!(entity=?e, "begin resizing {edges:?}");
                        start_grab_events.write(StartGrab::Resizing {
                            surface: *e,
                            seat: *seat_entity,
                            serial: None,
                            edges: *edges,
                            geometry: toplevel.geo.clone(),
                        });
                        ControlFlow::<()>::default()
                    });
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
        app.register_type::<DWayToplevel>();
        app.add_systems(
            Last,
            (
                update_window,
                receive_window_action_event.in_set(DWayServerSet::ProcessWindowAction),
            )
                .chain(),
        );
    }
}
