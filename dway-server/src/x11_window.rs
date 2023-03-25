use std::{ffi::OsString, time::Duration};

use bevy::prelude::*;
use dway_protocol::window::WindowState;
use failure::Fallible;
use smithay::{
    desktop::Window,
    reexports::{
        calloop::LoopHandle,
        wayland_server::{protocol::wl_surface::WlSurface, DisplayHandle, Resource},
    },
    utils::Scale,
    wayland::shell::xdg::ToplevelSurface,
    xwayland::{xwm::ResizeEdge, X11Wm, XWayland, XWaylandEvent, XWaylandSource, XwmHandler},
};

use crate::{
    components::{
        LogicalRect, PhysicalRect, SurfaceId, WaylandWindow, WindowIndex, WindowMark, WindowScale,
        WlSurfaceWrapper, X11Window, UUID,
    },
    cursor::Cursor,
    events::{
        CloseWindowRequest, ConfigureX11WindowNotify, ConfigureX11WindowRequest,
        CreateTopLevelEvent, CreateWindow, CreateX11WindowEvent, DestroyWindow, DestroyWlSurface,
        DestroyX11WindowEvent, MapOverrideX11Window, MapX11WindowRequest, MoveRequest, ResizeRequest,
        SetState, UnmapX11Window, X11WindowSetSurfaceEvent,
    },
    DWay,
    // wayland::{
    //     DWayState,
    // },
    DWayBackend,
    DWayServerComponent,
};

#[derive(Bundle)]
pub struct X11WindowBundle {
    pub mark: WindowMark,
    pub window: X11Window,
    pub uuid: UUID,
    pub id: SurfaceId,
}

pub fn create_x11_surface(
    mut events: EventReader<CreateX11WindowEvent>,
    mut window_index: ResMut<WindowIndex>,
    mut commands: Commands,
) {
    for CreateX11WindowEvent {
        window,
        is_override_redirect,
    } in events.iter()
    {
        let id = SurfaceId::from(window);
        let uuid = UUID::new();
        let wl_surface = window.wl_surface();
        let entity = *window_index.0.entry(id.clone()).or_insert_with(|| {
            let mut c = commands.spawn(X11WindowBundle {
                mark: WindowMark,
                window: X11Window(window.clone()),
                uuid,
                id: id.clone(),
            });
            if let Some(wl_surface) = wl_surface.as_ref() {
                c.insert(WlSurfaceWrapper(wl_surface.clone()));
            }
            let entity = c.id();
            info!(surface=?id,?entity,"create x11 window, surface: {:?}",wl_surface.as_ref().map(|s|s.id()));
            entity
        });
        if let Some(wl_surface) = wl_surface.as_ref() {
            window_index.0.insert(wl_surface.into(), entity);
        }
    }
}
pub fn map_x11_surface_notify(
    mut events: EventReader<X11WindowSetSurfaceEvent>,
    mut window_index: ResMut<WindowIndex>,
    window_query: Query<(Entity, &X11Window)>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let id = &e.0;
        if let Some((entity, surface)) = window_index.query(id, &window_query) {
            if let Some(wl_surface) = surface.wl_surface() {
                trace!(surface=?SurfaceId::from(&surface.0),surface=?SurfaceId::from(&wl_surface),?entity,"mapped x11 window to wl_surface");
                window_index.insert(SurfaceId::from(&wl_surface), entity);
                commands.entity(entity).insert(WlSurfaceWrapper(wl_surface));
            } else {
                error!(wl_surface=?surface,?entity,"no wl_surface");
            }
        }
    }
}
pub fn map_x11_window(
    mut events: EventReader<MapX11WindowRequest>,
    window_index: Res<WindowIndex>,
    windows: Query<(Entity, &X11Window, &PhysicalRect, Option<&WindowScale>)>,
) {
    for e in events.iter() {
        let id = &e.0;
        if let Some((entity, window, rect, scale)) = window_index.query(id, &windows) {
            let scale = scale.cloned().unwrap_or_default().0;
            window.set_mapped(true).unwrap();
            window
                .configure(Some(rect.to_f64().to_logical(scale).to_i32_round()))
                .unwrap();
            info!(surface=?id,?entity,"map x11 window request");
        }
    }
}
pub fn configure_request(
    mut events: EventReader<ConfigureX11WindowRequest>,
    window_index: Res<WindowIndex>,
    mut window_query: Query<(Entity, &X11Window, &mut PhysicalRect, Option<&WindowScale>)>,
    mut commands: Commands,
) {
    for ConfigureX11WindowRequest {
        window:id,
        x,
        y,
        w,
        h,
        reorder,
    } in events.iter()
    {
        if let Some((entity, window, mut rect, scale)) =
            window_index.query_mut(id, &mut window_query)
        {
            let mut geo = window.geometry();
            if let Some(w) = w {
                geo.size.w = *w as i32;
            }
            if let Some(h) = h {
                geo.size.h = *h as i32;
            }
            if let Some(x) = x {
                geo.loc.x = *x as i32;
            }
            if let Some(y) = y {
                geo.loc.y = *y as i32;
            }
            let _ = window.configure(geo);
            let physical_rect = geo.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
            info!(surface=?id,?entity,"configure x11 window request");
            rect.0 = physical_rect;
        }
    }
}
pub fn configure_notify(
    mut events: EventReader<ConfigureX11WindowNotify>,
    window_index: Res<WindowIndex>,
    mut windows_query: Query<(
        Entity,
        &mut PhysicalRect,
        Option<&WindowScale>,
        Option<&Parent>,
    )>,
    mut commands: Commands,
) {
    for ConfigureX11WindowNotify {
        window: id,
        geometry,
        above,
    } in events.iter()
    {
        if let Some((entity, mut rect, scale, parent)) =
            window_index.query_mut(id, &mut windows_query)
        {
            rect.0 = geometry.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
            if let Some(above) = above {
                if let Some(parent_entity) = window_index.get(&SurfaceId::X11(*above)) {
                    if let Ok(_) = windows_query.get(*parent_entity) {
                        commands.entity(entity).remove_parent();
                        commands.entity(*parent_entity).add_child(entity);
                    }
                } else {
                    error!(surface=?SurfaceId::X11(*above),"window entity not found");
                }
            } else {
                if let Some(_parent) = parent {
                    commands.entity(entity).remove_parent();
                }
            }
            info!(surface=?id,?entity,"configure x11 window notify");
        }
    }
}
pub fn unmap_x11_surface(
    mut events: EventReader<UnmapX11Window>,
    mut window_index: ResMut<WindowIndex>,
    surface_query: Query<(Entity, &WlSurfaceWrapper), With<WindowMark>>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let id = &e.0;
        if let Some((entity, surface)) = window_index.query(id, &surface_query) {
            info!(surface=?id,wl_surface=?SurfaceId::from(surface),?entity,"unmap x11 window");
            commands.entity(entity).remove::<WlSurfaceWrapper>();
            window_index.remove(&surface.into());
        }
    }
}
pub fn on_close_window_request(
    mut events: EventReader<CloseWindowRequest>,
    window_index: Res<WindowIndex>,
    window_query: Query<&X11Window, With<WindowMark>>,
) {
    for CloseWindowRequest(id) in events.iter() {
        if let Some(window) = window_index.query(id, &window_query) {
            if let Err(error) = window.close() {
                error!(%error,"failed to close x11 window");
            }
        }
    }
}
pub fn on_rect_changed(
    window_query: Query<(&LogicalRect, &X11Window), (With<WindowMark>, Changed<LogicalRect>)>,
) {
    for (rect, window) in window_query.iter() {
        info!(surface=?SurfaceId::from(&window.0),"configure x11 window: {:?}",rect);
        if let Err(error) = window.configure(Some(rect.0)) {
            error!(%error,"failed to configure x11 window");
        }
    }
}
pub fn on_state_changed(
    window_query: Query<(&WindowState, &X11Window), (With<WindowMark>, Changed<WindowState>)>,
) {
    for (window_state, window) in window_query.iter() {
        let result = if window.is_maximized() != (*window_state == WindowState::Maximized) {
            window.set_maximized(*window_state == WindowState::Maximized)
        } else if window.is_fullscreen() != (*window_state == WindowState::FullScreen) {
            window.set_fullscreen(*window_state == WindowState::FullScreen)
        } else if window.is_minimized() != (*window_state == WindowState::Minimized) {
            window.set_minimized(*window_state == WindowState::Minimized)
        } else {
            Ok(())
        };
        if let Err(e) = result {
            error!(error=%e,"failed to set state of x11 window");
        }
    }
}

impl XwmHandler for DWayServerComponent {
    fn xwm_state(&mut self, xwm: smithay::xwayland::xwm::XwmId) -> &mut smithay::xwayland::X11Wm {
        self.dway.xwm.as_mut().unwrap()
    }

    fn new_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(CreateWindow((&window).into()));
        self.dway.send_ecs_event(CreateX11WindowEvent {
            window,
            is_override_redirect: false,
        });
    }

    fn new_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(CreateWindow((&window).into()));
        self.dway.send_ecs_event(CreateX11WindowEvent {
            window,
            is_override_redirect: true,
        });
    }

    fn map_window_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(MapX11WindowRequest((&window).into()));
    }

    fn map_window_notify(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway
            .send_ecs_event(X11WindowSetSurfaceEvent((&window).into()));
    }

    fn mapped_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway
            .send_ecs_event(MapOverrideX11Window((&window).into()));
    }

    fn unmapped_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(UnmapX11Window((&window).into()));
    }

    fn destroyed_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(DestroyWlSurface((&window).into()));
    }

    fn configure_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        reorder: Option<smithay::xwayland::xwm::Reorder>,
    ) {
        self.dway.send_ecs_event(ConfigureX11WindowRequest {
            window: window.into(),
            x,
            y,
            w,
            h,
            reorder,
        });
    }

    fn configure_notify(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
        above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        self.dway.send_ecs_event(ConfigureX11WindowNotify {
            window: window.into(),
            geometry,
            above,
        });
    }

    fn maximize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::Maximized,
            unset: false,
        });
    }

    fn unmaximize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::Maximized,
            unset: true,
        });
    }

    fn fullscreen_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::FullScreen,
            unset: false,
        });
    }

    fn unfullscreen_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::FullScreen,
            unset: true,
        });
    }

    fn minimize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::Minimized,
            unset: false,
        });
    }

    fn unminimize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        self.dway.send_ecs_event(SetState {
            surface: (&window).into(),
            state: WindowState::Minimized,
            unset: true,
        });
    }

    fn resize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        let (top, bottom, left, right) = match resize_edge {
            ResizeEdge::Top => (true, false, false, false),
            ResizeEdge::Bottom => (false, true, false, false),
            ResizeEdge::Left => (false, false, true, false),
            ResizeEdge::TopLeft => (true, false, true, false),
            ResizeEdge::BottomLeft => (false, true, true, false),
            ResizeEdge::Right => (false, false, false, true),
            ResizeEdge::TopRight => (true, false, false, true),
            ResizeEdge::BottomRight => (false, true, false, true),
        };
        self.dway.send_ecs_event(ResizeRequest {
            surface: (&window).into(),
            top,
            bottom,
            left,
            right,
        });
    }

    fn move_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
    ) {
        self.dway.send_ecs_event(MoveRequest((&window).into()));
    }
}
pub fn init(
    dh: &DisplayHandle,
    handle: &LoopHandle<'static, DWayServerComponent>,
) -> (XWayland, Option<u32>) {
    let (xwayland, channel) = XWayland::new(&dh);
    let dh2 = dh.clone();
    let handle2 = handle.clone();
    let ret = handle.insert_source(channel, move |event, _, data| match event {
        XWaylandEvent::Ready {
            connection,
            client,
            client_fd: _,
            display,
        } => {
            info!("xwayland ready");
            data.dway.display_number = Some(display);
            let mut wm = X11Wm::start_wm(handle2.clone(), dh2.clone(), connection, client)
                .expect("Failed to attach X11 Window Manager");
            let cursor = Cursor::load();
            let image = cursor.get_image(1, Duration::ZERO);
            wm.set_cursor(
                &image.pixels_rgba,
                smithay::utils::Size::from((image.width as u16, image.height as u16)),
                smithay::utils::Point::from((image.xhot as u16, image.yhot as u16)),
            )
            .expect("Failed to set xwayland default cursor");
            data.dway.xwm = Some(wm);
        }
        XWaylandEvent::Exited => {
            warn!("xwayland exited");
            let _ = data.dway.xwm.take();
        }
    });
    if let Err(e) = ret {
        error!(
            "Failed to insert the XWaylandSource into the event loop: {}",
            e
        );
    }
    let display_number = match xwayland.start(
        handle.clone(),
        None,
        std::iter::empty::<(OsString, OsString)>(),
        |_| {
            info!("x11 client attached");
        },
    ) {
        Ok(o) => Some(o),
        Err(error) => {
            error!(%error,"Failed to start XWayland");
            None
        }
    };
    (xwayland, display_number)
}
