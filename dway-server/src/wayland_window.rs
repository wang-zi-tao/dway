use std::mem;

use bevy::prelude::*;
use dway_protocol::window::WindowState;
use failure::Fallible;
use smithay::{
    delegate_xdg_activation, delegate_xdg_shell,
    desktop::Window,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::Resource,
    },
    wayland::shell::xdg::{ToplevelStateSet, ToplevelSurface, XdgShellHandler},
};

use crate::{
    components::{
        LogicalRect, PhysicalRect, SurfaceId, WaylandWindow, WindowIndex, WindowMark,
        WlSurfaceWrapper, UUID,
    },
    events::{
        CloseWindowRequest, CreatePopup, CreateTopLevelEvent, CreateWindow, DestroyPopup,
        DestroyWlSurface, GrabPopup, MoveRequest, ResizeRequest, SetState, ShowWindowMenu,
        UpdatePopupPosition,
    },
    DWay,
    // wayland::{
    //     DWayState,
    // },
    DWayBackend,
};
#[derive(Bundle)]
pub struct WaylandSurfaceBundle {
    pub mark: WindowMark,
    pub window: WlSurfaceWrapper,
    pub uuid: UUID,
    pub id: SurfaceId,
}

#[derive(Bundle)]
pub struct WaylandWindowBundle {
    pub surface_bundle: WaylandSurfaceBundle,
    pub window: WaylandWindow,
}

pub fn create_top_level(
    mut create_top_level_event: EventReader<CreateTopLevelEvent>,
    mut window_index: ResMut<WindowIndex>,
    mut commands: Commands,
) {
    for e in create_top_level_event.iter() {
        let top_level = &e.0;
        let wl_surface = top_level.wl_surface();
        let id = SurfaceId::from(wl_surface);
        let uuid = UUID::new();
        window_index.0.entry(id.clone()).or_insert_with(|| {
            let entity = commands
                .spawn(WaylandWindowBundle {
                    window: WaylandWindow(Window::new(top_level.clone())),
                    surface_bundle: WaylandSurfaceBundle {
                        mark: WindowMark,
                        uuid,
                        id: id.clone(),
                        window: WlSurfaceWrapper(wl_surface.clone()),
                    },
                })
                .id();
            info!("create toplevel of {:?} on {:?}", id, entity);
            entity
        });
    }
}
pub fn destroy_wl_surface(
    mut create_top_level_event: EventReader<DestroyWlSurface>,
    mut window_index: ResMut<WindowIndex>,
    mut commands: Commands,
) {
    for e in create_top_level_event.iter() {
        let id = &e.0;
        if let Some(entity) = window_index.0.get(&id) {
            commands.entity(*entity).despawn_recursive();
        }
        window_index.0.remove(&id);
    }
}
pub fn on_close_window_request(
    mut events: EventReader<CloseWindowRequest>,
    window_index: Res<WindowIndex>,
    window_query: Query<&WaylandWindow, With<WindowMark>>,
) {
    for CloseWindowRequest(id) in events.iter() {
        if let Some(window) = window_index.get(id).and_then(|e| window_query.get(*e).ok()) {
            window.toplevel().send_close();
        }
    }
}
pub fn on_rect_changed(
    window_query: Query<(&LogicalRect, &WaylandWindow), (With<WindowMark>, Changed<LogicalRect>)>,
) {
    for (rect, window) in window_query.iter() {
        let toplevel = window.toplevel();
        let changed = toplevel.with_pending_state(|state| {
            let changed = state.size == Some(rect.0.size);
            if !changed {
                state.size = Some(rect.0.size);
            }
            changed
        });
        if changed {
            toplevel.send_configure();
        }
    }
}
pub fn on_state_changed(
    window_query: Query<(&WindowState, &WaylandWindow), (With<WindowMark>, Changed<WindowState>)>,
) {
    for (window_state, wayland_window) in window_query.iter() {
        let toplevel = wayland_window.toplevel();
        let changed=toplevel.with_pending_state(|toplevel_state|{
            let  mut changed=false;
            let old_states=mem::take(&mut toplevel_state.states);
            for state in old_states{
                let remove=match state{
                    smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Maximized => *window_state!=WindowState::Maximized,
                    smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::State::Fullscreen => *window_state!=WindowState::FullScreen,
                    _ => false,
                };
                if !remove{
                    toplevel_state.states.set(state);
                }else{
                    changed=true;
                }
            }
            match window_state{
                WindowState::Maximized => {
                    changed|=toplevel_state.states.set(xdg_toplevel::State::Maximized);
                },
                WindowState::FullScreen => {
                    changed|=toplevel_state.states.set(xdg_toplevel::State::Fullscreen);
                },
                _=>{}
            }
            changed
        });
        if changed {
            toplevel.send_configure();
        }
    }
}

delegate_xdg_shell!(DWay);
impl XdgShellHandler for DWay {
    fn xdg_shell_state(&mut self) -> &mut smithay::wayland::shell::xdg::XdgShellState {
        &mut self.xdg_shell
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        trace!("new toplevel {:?}", surface.wl_surface().id());
        self.send_ecs_event(CreateWindow((&surface).into()));
        self.send_ecs_event(CreateTopLevelEvent(surface));
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        trace!("new popup {:?}", surface.wl_surface().id());
        self.send_ecs_event(CreateWindow((&surface).into()));
        self.send_ecs_event(CreatePopup(surface, positioner));
    }

    fn grab(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        self.send_ecs_event(GrabPopup(surface.into()))
    }

    fn new_client(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn client_pong(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn move_request(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        self.send_ecs_event(MoveRequest(SurfaceId::from(&surface)));
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
        let (top, bottom, left, right) = match edges {
            ResizeEdge::Top => (true, false, false, false),
            ResizeEdge::Bottom => (false, true, false, false),
            ResizeEdge::Left => (false, false, true, false),
            ResizeEdge::TopLeft => (true, false, true, false),
            ResizeEdge::BottomLeft => (false, true, true, false),
            ResizeEdge::Right => (false, false, false, true),
            ResizeEdge::TopRight => (true, false, false, true),
            ResizeEdge::BottomRight => (false, true, false, true),
            o => {
                warn!("unknown resize edge: {o:?}");
                return;
            }
        };
        self.send_ecs_event(ResizeRequest {
            surface: (&surface).into(),
            top,
            bottom,
            left,
            right,
        });
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        self.send_ecs_event(SetState {
            surface: (&surface).into(),
            state: WindowState::Maximized,
            unset: false,
        });
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        self.send_ecs_event(SetState {
            surface: (&surface).into(),
            state: WindowState::Maximized,
            unset: true,
        });
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        self.send_ecs_event(SetState {
            surface: (&surface).into(),
            state: WindowState::FullScreen,
            unset: false,
        });
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        self.send_ecs_event(SetState {
            surface: (&surface).into(),
            state: WindowState::FullScreen,
            unset: true,
        });
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        self.send_ecs_event(SetState {
            surface: (&surface).into(),
            state: WindowState::Minimized,
            unset: false,
        });
    }

    fn show_window_menu(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        location: smithay::utils::Point<i32, smithay::utils::Logical>,
    ) {
        self.send_ecs_event(ShowWindowMenu(surface.into()));
    }

    fn ack_configure(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        configure: smithay::wayland::shell::xdg::Configure,
    ) {
    }

    fn reposition_request(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
        token: u32,
    ) {
        self.send_ecs_event(UpdatePopupPosition {
            surface_id: surface.into(),
            positioner,
            token,
        })
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.send_ecs_event(DestroyWlSurface(surface.into()));
    }

    fn popup_destroyed(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface) {
        self.send_ecs_event(DestroyPopup((&surface).into()));
        self.send_ecs_event(DestroyWlSurface(surface.into()));
    }
}
