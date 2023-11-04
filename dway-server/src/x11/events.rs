use anyhow::{anyhow, Result};
use bevy::prelude::DespawnRecursiveExt;
use scopeguard::defer;
use thiserror::Error;
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        AtomEnum, ConfigWindow, ConfigureWindowAux, ConnectionExt, CreateWindowAux, EventMask,
        PropMode, WindowClass,
    },
    rust_connection::ConnectionError,
    COPY_DEPTH_FROM_PARENT,
};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::{
        grab::{Grab, ResizeEdges},
        seat::WlSeat,
    },
    prelude::*,
    util::rect::IRect,
    x11::{
        screen::{XScreen, XScreenBundle},
        util::geo_to_irect,
        window::{MappedXWindow, XWindow, XWindowAttachSurface, XWindowBundle, XWindowSurfaceRef},
        DWayXWaylandStoped, XDisplayHasWindow,
    },
    xdg::DWayWindow,
};

use super::{DWayHasXWayland, DWayXWaylandReady, XWaylandDisplay, XWaylandDisplayWrapper};

#[derive(Error, Debug)]
pub enum XWaylandError {
    #[error("x11 window {} not exists", _0)]
    WindowNotExists(u32),
    #[error("x11 window entity {:?} not valid", _0)]
    InvalidWindowEntity(Entity),
    #[error("xwayland connection error: {}", _0)]
    ConnectionError(ConnectionError),
}

pub fn process_x11_event(
    dway: &mut DWay,
    display_entity: Entity,
    x: &mut XWaylandDisplay,
    event: x11rb::protocol::Event,
) -> Result<()> {
    use XWaylandError::*;
    let Some(connection) = x.connection.upgrade() else {
        return Err(anyhow!("xwayland connection has droped"));
    };
    let (rust_connection, atoms) = &*connection;
    debug!(entity = ?display_entity,"xwayland event: {event:?}");
    let _span = span!(Level::ERROR,"xwayland event",entity = ?display_entity).entered();
    match event {
        x11rb::protocol::Event::Unknown(_) => todo!(),
        x11rb::protocol::Event::Error(e) => {
            error!("x11 error: {:?}", e);
        }
        x11rb::protocol::Event::ButtonPress(_) => todo!(),
        x11rb::protocol::Event::ButtonRelease(_) => todo!(),
        x11rb::protocol::Event::CirculateNotify(_) => todo!(),
        x11rb::protocol::Event::CirculateRequest(_) => todo!(),
        x11rb::protocol::Event::ClientMessage(e) => {
            if let Some(reply) = rust_connection.get_atom_name(e.type_)?.reply_unchecked()? {
                debug!(
                    event = std::str::from_utf8(&reply.name).unwrap(),
                    message = ?e,
                    "got X11 client event message",
                );
            }
            match e.type_ {
                t if t == atoms.WL_SURFACE_ID => {
                    let world = dway.world_mut();
                    let wid = e.data.as_data32()[0];
                    debug!(entity=?e,xwindow=%e.window,"attach surface: {wid}");
                    let xwindow_entity = x.find_window(e.window)?;
                    let mut xwindow = world
                        .get_mut::<XWindow>(xwindow_entity)
                        .ok_or_else(|| InvalidWindowEntity(xwindow_entity))?;
                    xwindow.surface_id = Some(wid);
                    world.entity_mut(xwindow_entity).insert(MappedXWindow);
                }
                t if t == atoms.WM_CHANGE_STATE => {
                    todo!()
                }
                t if t == atoms._NET_WM_STATE => {
                    debug!("message type: _NET_WM_STATE");
                    // TODO
                }
                t if t == atoms._NET_WM_MOVERESIZE => {
                    debug!("message type: _NET_WM_MOVERESIZE");
                    let xwindow_entity = x.find_window(e.window)?;
                    let surface_entity = dway
                        .get_mut::<XWindowSurfaceRef>(xwindow_entity)
                        .and_then(|r| r.get());
                    if let Some(surface_entity) = surface_entity {
                        let Some(rect) = dway.get::<Geometry>(surface_entity).map(|r| r.geometry)
                        else {
                            bail!("surface has no geometry component");
                        };
                        let pos = rect.pos();
                        dway.query::<(&mut Grab, &mut WlSeat), _, _>(
                            display_entity,
                            |(mut grab, mut seat)| {
                                let data = e.data.as_data32();
                                match data[2] {
                                    x @ 0..=7 => {
                                        let edges = match x {
                                            0 => ResizeEdges::TOP | ResizeEdges::LEFT,
                                            1 => ResizeEdges::TOP,
                                            2 => ResizeEdges::TOP | ResizeEdges::RIGHT,
                                            3 => ResizeEdges::RIGHT,
                                            4 => ResizeEdges::BUTTOM | ResizeEdges::RIGHT,
                                            5 => ResizeEdges::BUTTOM,
                                            6 => ResizeEdges::BUTTOM | ResizeEdges::LEFT,
                                            7 => ResizeEdges::LEFT,
                                            _ => unreachable!(),
                                        };
                                        *grab = Grab::Resizing {
                                            surface: surface_entity,
                                            serial: 0,
                                            edges,
                                            relative: rect.pos()
                                                - seat.pointer_position.unwrap_or_default(),
                                            origin_rect: rect,
                                        };
                                        debug!("xwindow start resizing");
                                        seat.disable();
                                    }
                                    8 => {
                                        *grab = Grab::Moving {
                                            surface: surface_entity,
                                            serial: 0,
                                            relative: pos
                                                - seat.pointer_position.unwrap_or_default(),
                                        };
                                        seat.disable();
                                        debug!("xwindow start moving");
                                    }
                                    _ => {} // ignore keyboard moves/resizes for now
                                }
                            },
                        );
                    }
                }
                t => {
                    debug!(
                        "Unhandled client msg of type {:?}",
                        String::from_utf8(
                            rust_connection
                                .get_atom_name(t)?
                                .reply_unchecked()?
                                .unwrap()
                                .name
                        )
                        .ok()
                    )
                }
            }
        }
        x11rb::protocol::Event::ColormapNotify(_) => todo!(),
        x11rb::protocol::Event::ConfigureNotify(r) => {
            // TODO map onto
            if let Ok(e) = x.find_window(r.window) {
                let surface_ref = dway
                    .query::<(&XWindow, &mut Geometry, Option<&XWindowSurfaceRef>), _, _>(
                        e,
                        |(_xwindow, mut geometry, surface_ref)| {
                            geometry.set_x(r.x as i32);
                            geometry.set_y(r.y as i32);
                            surface_ref.and_then(|r| r.get())
                        },
                    );
                if let Some(surface_ref) = surface_ref {
                    if let Some(mut geometry) = dway.entity_mut(surface_ref).get_mut::<Geometry>() {
                        geometry.set_x(r.x as i32);
                        geometry.set_y(r.y as i32);
                    }
                }
                debug!(entity=?e,xwindow=%r.window,"configure window");
            }
        }
        x11rb::protocol::Event::ConfigureRequest(r) => {
            let world = dway.world_mut();
            let window_entity = x.find_window(r.window)?;
            if r.value_mask & (ConfigWindow::WIDTH | ConfigWindow::HEIGHT)
                != ConfigWindow::default()
            {
                let mut geo = world
                    .get_mut::<Geometry>(window_entity)
                    .ok_or(InvalidWindowEntity(window_entity))?;
                if r.value_mask.contains(ConfigWindow::WIDTH) {
                    geo.set_width(r.width as i32);
                }
                if r.value_mask.contains(ConfigWindow::HEIGHT) {
                    geo.set_width(r.height as i32);
                }
                let rect = geo.geometry;
                let aux = ConfigureWindowAux::default()
                    .x(rect.x())
                    .y(rect.y())
                    .width(rect.width() as u32)
                    .height(rect.height() as u32);
                rust_connection.configure_window(r.window, &aux)?;
                rust_connection.flush()?;
                debug!(entity=?window_entity,xwindow=%r.window,"configure request");
            }
        }
        x11rb::protocol::Event::CreateNotify(c) => {
            let world = dway.world_mut();
            let xwindow = XWindow::new(
                connection.clone(),
                c.window,
                Some(c.parent),
                c.override_redirect,
                x.screen_windows.contains(&c.parent),
            );
            let rect = geo_to_irect(rust_connection.get_geometry(c.window)?.reply()?);
            let bundle = XWindowBundle {
                xwindow,
                geometry: Geometry::new(rect),
                global_geometry: GlobalGeometry::new(rect),
            };
            let entity_mut = if Some(c.window) == x.wm_window {
                let mut entity_mut = world.entity_mut(display_entity);
                entity_mut.insert(bundle);
                entity_mut
            } else {
                let mut entity_mut =
                    world.spawn((bundle, Name::from(format!("xwindow:{}", c.window))));
                let parent_entity = if let Ok(parent_entity) = x.find_window(c.parent) {
                    debug!(xwindow=%c.window,"set parent to {:?}", parent_entity);
                    parent_entity
                } else {
                    error!("parent window {} not found", c.parent);
                    display_entity
                };
                entity_mut.set_parent(parent_entity);
                entity_mut
            };
            let entity = entity_mut.id();
            debug!(entity=?entity,xwindow=%c.window,"create window");
            x.windows_entitys.insert(c.window, entity_mut.id());
            dway.connect::<XDisplayHasWindow>(display_entity, entity);
        }
        x11rb::protocol::Event::DestroyNotify(e) => {
            let world = dway.world_mut();
            if let Some(entity) = x.windows_entitys.remove(&e.window) {
                debug!(entity=?entity,xwindow=%e.window,"destroy window");
                world.entity_mut(entity).despawn_recursive();
            }
        }
        x11rb::protocol::Event::EnterNotify(_) => todo!(),
        x11rb::protocol::Event::Expose(_) => todo!(),
        x11rb::protocol::Event::FocusIn(_) => {}
        x11rb::protocol::Event::FocusOut(_) => {}
        x11rb::protocol::Event::GeGeneric(_) => todo!(),
        x11rb::protocol::Event::GraphicsExposure(_) => todo!(),
        x11rb::protocol::Event::GravityNotify(_) => todo!(),
        x11rb::protocol::Event::KeyPress(_) => todo!(),
        x11rb::protocol::Event::KeyRelease(_) => todo!(),
        x11rb::protocol::Event::KeymapNotify(_) => todo!(),
        x11rb::protocol::Event::LeaveNotify(_) => todo!(),
        x11rb::protocol::Event::MapNotify(r) => {
            let world = dway.world_mut();
            let window_entity = x.find_window(r.window)?;
            let xwindow = world
                .get::<XWindow>(window_entity)
                .ok_or(InvalidWindowEntity(window_entity))?;
            if let Some(parent) = xwindow.parent_window {
                x11rb::wrapper::ConnectionExt::change_property32(
                    rust_connection,
                    PropMode::APPEND,
                    parent,
                    atoms._NET_CLIENT_LIST,
                    AtomEnum::WINDOW,
                    &[r.window],
                )?;
                x11rb::wrapper::ConnectionExt::change_property32(
                    rust_connection,
                    PropMode::APPEND,
                    parent,
                    atoms._NET_CLIENT_LIST_STACKING,
                    AtomEnum::WINDOW,
                    &[r.window],
                )?;
                rust_connection.flush()?;
            }
            debug!(entity=?window_entity,xwindow=%r.window,"mapped window");
        }
        x11rb::protocol::Event::MapRequest(r) => {
            let world = dway.world_mut();
            let window_entity = x.find_window(r.window)?;
            let rect = world
                .get::<Geometry>(window_entity)
                .ok_or(InvalidWindowEntity(window_entity))?
                .geometry;
            defer! {
                let _ = rust_connection.ungrab_server();
            };
            rust_connection.grab_server()?;
            let frame_window = rust_connection.generate_id()?;
            let xwindow = world
                .get::<XWindow>(window_entity)
                .ok_or(InvalidWindowEntity(window_entity))?;
            if let Some(parent) = xwindow.parent_window {
                let aux = CreateWindowAux::default()
                    .event_mask(EventMask::SUBSTRUCTURE_NOTIFY | EventMask::SUBSTRUCTURE_REDIRECT);
                rust_connection.create_window(
                    COPY_DEPTH_FROM_PARENT,
                    frame_window,
                    parent,
                    rect.x() as i16,
                    rect.y() as i16,
                    rect.width() as u16,
                    rect.height() as u16,
                    xwindow.boarder_width as u16,
                    WindowClass::INPUT_OUTPUT,
                    x11rb::COPY_FROM_PARENT,
                    &aux,
                )?;
                // let cookie = rust_connection.reparent_window(r.window, parent, 0, 0)?;
                rust_connection.map_window(r.window)?;
                rust_connection.flush()?;
            }
            debug!(entity=?window_entity,xwindow=%r.window,"map request");
        }
        x11rb::protocol::Event::MappingNotify(_) => {}
        x11rb::protocol::Event::MotionNotify(_) => todo!(),
        x11rb::protocol::Event::NoExposure(_) => todo!(),
        x11rb::protocol::Event::PropertyNotify(e) => {
            let world = dway.world_mut();
            let entity = x.find_window(e.window)?;
            let mut window = world.get_mut::<XWindow>(entity).unwrap();
            window.update_property(x, Some(e.atom))?;
        }
        x11rb::protocol::Event::ReparentNotify(_) => todo!(),
        x11rb::protocol::Event::ResizeRequest(_) => todo!(),
        x11rb::protocol::Event::SelectionClear(_) => todo!(),
        x11rb::protocol::Event::SelectionNotify(_) => todo!(),
        x11rb::protocol::Event::SelectionRequest(_) => todo!(),
        x11rb::protocol::Event::UnmapNotify(r) => {
            let world = dway.world_mut();
            let window_entity = x.find_window(r.window)?;
            defer! {
                let _ = rust_connection.ungrab_server();
            };
            rust_connection.grab_server()?;
            debug!("unmap window {} at {:?}", r.window, window_entity);
            world
                .entity_mut(window_entity)
                .remove::<(DWayWindow, MappedXWindow)>();
            if let Some(surface_entity) = world
                .get::<XWindowSurfaceRef>(window_entity)
                .and_then(|r| r.get())
            {
                world.entity_mut(surface_entity).remove::<DWayWindow>();
                world.send_event(Destroy::<DWayWindow>::new(surface_entity));
                dway.disconnect_all::<XWindowAttachSurface>(window_entity);
            }
        }
        x11rb::protocol::Event::VisibilityNotify(_) => todo!(),
        x11rb::protocol::Event::ShapeNotify(_) => todo!(),
        x11rb::protocol::Event::XfixesCursorNotify(_) => todo!(),
        x11rb::protocol::Event::XfixesSelectionNotify(_) => todo!(),
        _ => todo!(),
    }
    Ok(())
}

pub fn dispatch_x11_events(world: &mut World) {
    let display_list = world
        .query::<(Entity, &XWaylandDisplayWrapper, &Parent)>()
        .iter(world)
        .map(|(entity, display, parent)| (entity, display.clone(), parent.get()))
        .collect::<Vec<_>>();
    display_list
        .into_iter()
        .for_each(|(display_entity, display, dway_entity)| {
            let mut x = display.inner.lock().unwrap();
            DWay::with(world, |dway| {
                for event in x.channel.clone().try_iter() {
                    let result = (|| {
                        let event = match event {
                            crate::x11::XWaylandThreadEvent::CreateConnection(
                                connection,
                                wm_window,
                            ) => {
                                let _span = span!(Level::ERROR,"xwayland",entity = ?display_entity)
                                    .entered();
                                let Some(connection_arc) = connection.upgrade() else {
                                    return Ok(());
                                };
                                x.connection = connection;
                                x.wm_window = Some(wm_window);
                                let rust_connection = &connection_arc.0;
                                for screen in &rust_connection.setup().roots {
                                    let rect = IRect::new(
                                        0,
                                        0,
                                        screen.width_in_pixels as i32,
                                        screen.height_in_pixels as i32,
                                    );
                                    let entity = dway
                                        .spawn((
                                            Name::new(format!("screen:{}", screen.root)),
                                            XScreenBundle {
                                                screen: XScreen {
                                                    raw: screen.clone(),
                                                },
                                                window: XWindow::new(
                                                    connection_arc.clone(),
                                                    screen.root,
                                                    None,
                                                    false,
                                                    false,
                                                ),
                                                geometry: Geometry::new(rect),
                                                global_geometry: GlobalGeometry::new(rect),
                                            },
                                        ))
                                        .set_parent(display_entity)
                                        .id();
                                    debug!("add root window {} at {:?}", screen.root, entity);
                                    x.windows_entitys.insert(screen.root, entity);
                                    x.screen_windows.insert(screen.root);
                                }
                                dway.send_event(DWayXWaylandReady::new(dway_entity));
                                dway.connect::<DWayHasXWayland>(dway_entity, display_entity);
                                return Ok(());
                            }
                            crate::x11::XWaylandThreadEvent::XWaylandEvent(event) => event,
                            crate::x11::XWaylandThreadEvent::Disconnect(e) => {
                                dway.despawn_tree(display_entity);
                                info!(cause=?e,"despawn xwayland on {display_entity:?}");
                                dway.send_event(DWayXWaylandStoped::new(dway_entity));
                                dway.disconnect::<DWayHasXWayland>(dway_entity, display_entity);
                                return Ok(());
                            }
                        };
                        process_x11_event(dway, display_entity, &mut x, event)
                    })();
                    if let Err(error) = result {
                        error!(%error);
                    }
                }
            });
        });
}

pub fn flush_xwayland(
    xwayland_query: Query<(Entity, &XWaylandDisplayWrapper)>,
    mut commands: Commands,
) {
    xwayland_query.for_each(|(entity, x)| {
        let guard = x.lock().unwrap();
        let Some(connection) = guard.connection.upgrade() else {
            commands.entity(entity).despawn_recursive();
            return;
        };
        if let Err(e) = connection.0.flush() {
            error!(entity=?entity,"failed to flush xwayland connection: {e}");
        };
    });
}
