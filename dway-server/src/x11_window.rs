use std::time::Duration;

use bevy::prelude::*;
use dway_protocol::window::WindowState;
use failure::Fallible;
use smithay::{
    desktop::Window,
    reexports::{
        calloop::LoopHandle,
        wayland_server::{DisplayHandle, Resource},
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
        DestroyX11WindowEvent, MapOverrideX11Window, MapX11Window, MoveRequest, ResizeRequest,
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
    for e in events.iter() {
        let x11_surface = &e.window;
        let id = SurfaceId::from(x11_surface);
        let uuid = UUID::new();
        window_index.0.entry(id.clone()).or_insert_with(|| {
            commands
                .spawn(X11WindowBundle {
                    mark: WindowMark,
                    window: X11Window(x11_surface.clone()),
                    uuid,
                    id: id.clone(),
                })
                .id()
        });
    }
}
pub fn map_x11_surface(
    mut events: EventReader<X11WindowSetSurfaceEvent>,
    window_index: Res<WindowIndex>,
    windows: Query<(Entity, &X11Window)>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let id = &e.0;
        if let Some((entity, surface)) = window_index.get(id).and_then(|&e| windows.get(e).ok()) {
            if let Some(wl_surface) = surface.wl_surface() {
                commands.entity(entity).insert(WlSurfaceWrapper(wl_surface));
            }
        }
    }
}
pub fn configure_request(
    mut events: EventReader<ConfigureX11WindowRequest>,
    window_index: Res<WindowIndex>,
    windows: Query<(Entity, &X11Window, Option<&WindowScale>)>,
    mut commands: Commands,
) {
    for ConfigureX11WindowRequest {
        window,
        x,
        y,
        w,
        h,
        reorder,
    } in events.iter()
    {
        if let Some((entity, x11_surface, scale)) =
            window_index.get(window).and_then(|&e| windows.get(e).ok())
        {
            let mut geo = x11_surface.geometry();
            if let Some(w) = w {
                geo.size.w = *w as i32;
            }
            if let Some(h) = h {
                geo.size.h = *h as i32;
            }
            let physical_rect = geo.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
            commands.entity(entity).insert(PhysicalRect(physical_rect));
        }
    }
}
pub fn configure_notify(
    mut events: EventReader<ConfigureX11WindowNotify>,
    window_index: Res<WindowIndex>,
    windows: Query<(Entity, Option<&WindowScale>)>,
    mut commands: Commands,
) {
    for ConfigureX11WindowNotify {
        window,
        geometry,
        above,
    } in events.iter()
    {
        if let Some((entity, scale)) = window_index.get(window).and_then(|&e| windows.get(e).ok()) {
            let physical_rect =
                geometry.to_physical_precise_round(scale.cloned().unwrap_or_default().0);
            commands.entity(entity).insert(PhysicalRect(physical_rect));
        }
    }
}
pub fn unmap_x11_surface(
    mut events: EventReader<UnmapX11Window>,
    window_index: Res<WindowIndex>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let id = &e.0;
        if let Some(entity) = window_index.get(id) {
            commands.entity(*entity).remove::<WlSurfaceWrapper>();
        }
    }
}
pub fn on_close_window_request(
    mut events: EventReader<CloseWindowRequest>,
    window_index: Res<WindowIndex>,
    window_query: Query<&X11Window, With<WindowMark>>,
) {
    for CloseWindowRequest(id) in events.iter() {
        if let Some(window) = window_index.get(id).and_then(|e| window_query.get(*e).ok()) {
            window.close();
        }
    }
}
pub fn on_rect_changed(
    window_query: Query<(&LogicalRect, &X11Window), (With<WindowMark>, Changed<LogicalRect>)>,
) {
    for (rect, window) in window_query.iter() {
        window.configure(Some(rect.0));
    }
}
pub fn on_state_changed(
    window_query: Query<(&WindowState, &X11Window), (With<WindowMark>, Changed<WindowState>)>,
) {
    for (window_state, window) in window_query.iter() {
        if window.is_maximized() != (*window_state == WindowState::Maximized) {
            window.set_maximized(*window_state == WindowState::Maximized);
        }
        if window.is_fullscreen() != (*window_state == WindowState::FullScreen) {
            window.set_fullscreen(*window_state == WindowState::FullScreen);
        }
        if window.is_minimized() != (*window_state == WindowState::Minimized) {
            window.set_minimized(*window_state == WindowState::Minimized);
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
        self.dway.send_ecs_event(MapX11Window((&window).into()));
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
        self.dway.send_ecs_event(DestroyWindow((&window).into()));
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
pub fn init(dh: &DisplayHandle, handle: &LoopHandle<'static, DWayServerComponent>)->XWayland {
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
    xwayland
}
