use self::{screen::XScreen, window::XWindowSurfaceRef};
use super::*;
use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::grab::{SurfaceGrabKind, WlSurfacePointerState},
    prelude::*,
    wl::surface::{ClientHasSurface, WlSurface},
    xdg::{
        toplevel::{DWayToplevel, PinedWindow},
        DWayWindow,
    },
};

graph_query!(
XWindowGraph=>[
    surface=<(&'static Geometry, &'static mut WlSurfacePointerState, Option<&'static PinedWindow> ),With<DWayToplevel>>,
    xwindow=&'static mut XWindow,
    client=Entity,
]=>{
    path=surface-[XWindowAttachSurface]->xwindow,
    seat_path=surface-[ClientHasSurface]->client,
});

pub fn process_window_action_events(
    mut events: EventReader<WindowAction>,
    mut query_graph: XWindowGraph,
) {
    for event in events.read() {
        match (|| {
            match event {
                WindowAction::Close(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| ControlFlow::Return(window.close()))
                        .transpose()?;
                }
                WindowAction::Maximize(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom_hor = window.atoms()._NET_WM_STATE_MAXIMIZED_HORZ;
                            let atom_ver = window.atoms()._NET_WM_STATE_MAXIMIZED_VERT;
                            ControlFlow::Return(
                                window
                                    .change_net_state(atom_hor, true)
                                    .and(window.change_net_state(atom_ver, true)),
                            )
                        })
                        .transpose()?;
                }
                WindowAction::UnMaximize(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom_hor = window.atoms()._NET_WM_STATE_MAXIMIZED_HORZ;
                            let atom_ver = window.atoms()._NET_WM_STATE_MAXIMIZED_VERT;
                            ControlFlow::Return(
                                window
                                    .change_net_state(atom_hor, false)
                                    .and(window.change_net_state(atom_ver, false)),
                            )
                        })
                        .transpose()?;
                }
                WindowAction::Fullscreen(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom = window.atoms()._NET_WM_STATE_FULLSCREEN;
                            ControlFlow::Return(window.change_net_state(atom, true))
                        })
                        .transpose()?;
                }
                WindowAction::UnFullscreen(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom = window.atoms()._NET_WM_STATE_FULLSCREEN;
                            ControlFlow::Return(window.change_net_state(atom, false))
                        })
                        .transpose()?;
                }
                WindowAction::Minimize(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom = window.atoms()._NET_WM_STATE_HIDDEN;
                            ControlFlow::Return(window.change_net_state(atom, true))
                        })
                        .transpose()?;
                }
                WindowAction::UnMinimize(e) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            let atom = window.atoms()._NET_WM_STATE_HIDDEN;
                            ControlFlow::Return(window.change_net_state(atom, false))
                        })
                        .transpose()?;
                }
                WindowAction::SetRect(e, rect) => {
                    query_graph
                        .for_each_path_mut_from(*e, |_, window| {
                            ControlFlow::Return(window.set_rect(*rect))
                        })
                        .transpose()?;
                }
                WindowAction::RequestMove(e) => {
                    query_graph.for_each_seat_path_mut_from(
                        *e,
                        |(_geo, surface_pointer_state, pinned), seat_entity| {
                            if pinned.is_some() {
                                return ControlFlow::<()>::default();
                            }
                            let mouse_pos = surface_pointer_state.mouse_pos;
                            surface_pointer_state.set_grab(SurfaceGrabKind::Move {
                                mouse_pos,
                                seat: *seat_entity,
                                serial: None,
                            });
                            ControlFlow::<()>::default()
                        },
                    );
                }
                WindowAction::RequestResize(e, edges) => {
                    query_graph.for_each_seat_path_mut_from(
                        *e,
                        |(geo, surface_pointer_state, pinned), seat_entity| {
                            if pinned.is_some() {
                                return ControlFlow::<()>::default();
                            }
                            surface_pointer_state.set_grab(SurfaceGrabKind::Resizing {
                                seat: *seat_entity,
                                serial: None,
                                edges: *edges,
                                geo: geo.geometry,
                            });
                            ControlFlow::<()>::default()
                        },
                    );
                }
            }
            Result::<_>::Ok(())
        })() {
            Ok(o) => o,
            Err(e) => {
                error!("{}: {e}", "failed to apply window action");
                continue;
            }
        }
    }
}

graph_query!(
XWindowUpdateGraph=>[
    surface=<Option<&'static mut DWayToplevel>,With<DWayWindow>>,
    xwindow=<&'static mut XWindow, Or<(Changed<XWindow>, Added<MappedXWindow>)>>,
]=>{
    path=xwindow<-[XWindowAttachSurface]-surface,
});

pub fn update_xwindow(mut graph: XWindowUpdateGraph) {
    graph.for_each_path_mut(|xwindow, toplevel| {
        if let Some(toplevel) = toplevel {
            let decorated = xwindow.is_decorated();
            if decorated != toplevel.decorated {
                toplevel.decorated = true;
            }
            if xwindow.title != toplevel.title {
                toplevel.title = xwindow.title.clone();
            }
            // TODO
        }
        ControlFlow::<()>::Continue
    });
}

pub fn x11_window_attach_wl_surface(
    xwindow_query: Query<
        (
            Entity,
            &XWindow,
            &XDisplayRef,
            &Geometry,
            &GlobalGeometry,
            Option<&XWindowSurfaceRef>,
        ),
        (
            Without<WlSurface>,
            Without<XScreen>,
            With<MappedXWindow>,
            Without<XWaylandDisplayWrapper>,
        ),
    >,
    xdisplay_query: Query<(&XWaylandDisplayWrapper, &Parent)>,
    wl_query: Query<&DWayServer>,
    mut event_writter: EventWriter<Insert<DWayWindow>>,
    mut commands: Commands,
) {
    for (xwindow_entity, xwindow, display_ref, geometry, global_geometry, attached) in
        xwindow_query.iter()
    {
        if attached.map(|r| r.get().is_some()).unwrap_or_default() {
            continue;
        }
        if let Some(wid) = xwindow.surface_id {
            let Some((xdisplay_wrapper, wl_entity)) =
                display_ref.get().and_then(|e| xdisplay_query.get(e).ok())
            else {
                continue;
            };
            let Ok(dway) = wl_query.get(wl_entity.get()) else {
                continue;
            };
            let xdisplay = xdisplay_wrapper.lock().unwrap();
            let Ok(wl_surface) = xdisplay
                .client
                .clone()
                .object_from_protocol_id::<wl_surface::WlSurface>(&dway.display.handle(), wid)
            else {
                continue;
            };
            let wl_surface_entity = DWay::get_entity(&wl_surface);
            commands.add(ConnectCommand::<XWindowAttachSurface>::new(
                xwindow_entity,
                wl_surface_entity,
            ));
            let mut entity_mut = commands.entity(wl_surface_entity);
            entity_mut.insert((
                geometry.clone(),
                global_geometry.clone(),
                DWayWindow::default(),
            ));
            if xwindow.is_toplevel {
                entity_mut.insert(DWayToplevel::default());
            }
            event_writter.send(Insert::new(wl_surface_entity));
            commands.entity(xwindow_entity).insert(MappedXWindow);
            debug!(
                "xwindow {:?} attach wl_surface {:?}",
                xwindow_entity, wl_surface_entity
            );
        }
    }
}
