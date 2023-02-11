pub mod cursor;
pub mod focus;
pub mod grabs;
pub mod inputs;
pub mod render;
pub mod shell;
pub mod surface;
pub mod x11;

use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    collections::HashMap,
    ffi::OsString,
    os::unix::io::{AsRawFd, OwnedFd},
    process::Command,
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bevy_input::{keyboard::KeyboardInput, mouse::MouseButtonInput, prelude::MouseButton};
use bevy_math::Vec2;
use crossbeam_channel::{Receiver, Sender};
use failure::{format_err, Fallible};
use slog::{debug, error, info, trace, warn};
use smithay::{
    backend::renderer::{
        damage::DamageTrackedRenderer, utils::on_commit_buffer_handler, Frame, Renderer,
    },
    delegate_compositor, delegate_data_device, delegate_fractional_scale,
    delegate_input_method_manager, delegate_keyboard_shortcuts_inhibit, delegate_layer_shell,
    delegate_output, delegate_presentation, delegate_primary_selection, delegate_seat,
    delegate_shm, delegate_tablet_manager, delegate_text_input_manager, delegate_viewporter,
    delegate_virtual_keyboard_manager, delegate_xdg_activation, delegate_xdg_decoration,
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, space::SpaceElement,
        utils::with_surfaces_surface_tree, PopupKeyboardGrab, PopupKind, PopupManager,
        PopupPointerGrab, PopupUngrabStrategy, Space, Window, WindowSurfaceType,
    },
    input::{
        keyboard::XkbConfig,
        pointer::{CursorImageStatus, Focus},
        Seat, SeatHandler, SeatState,
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, PostAction},
        wayland_protocols::xdg::{
            decoration::{
                self as xdg_decoration,
                zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
            },
            shell::server::xdg_toplevel,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle, Resource,
        },
        x11rb::protocol::xfixes::get_cursor_image_and_name,
    },
    utils::{Clock, Logical, Monotonic, Physical, Point, Rectangle, Scale, Size, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, with_surface_tree_downward,
            with_surface_tree_upward, CompositorHandler, CompositorState, TraversalAction,
        },
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        fractional_scale::{FractionScaleHandler, FractionalScaleManagerState},
        input_method::{InputMethodManagerState, InputMethodSeat},
        keyboard_shortcuts_inhibit::{
            KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
        },
        output::OutputManagerState,
        presentation::PresentationState,
        primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
        seat::WaylandFocus,
        shell::{
            wlr_layer::{WlrLayerShellHandler, WlrLayerShellState},
            xdg::{
                decoration::{XdgDecorationHandler, XdgDecorationState},
                Configure, SurfaceCachedState, ToplevelSurface, XdgPopupSurfaceData,
                XdgPopupSurfaceRoleAttributes, XdgShellHandler, XdgShellState,
                XdgToplevelSurfaceData, XdgToplevelSurfaceRoleAttributes,
            },
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
        tablet_manager::TabletSeatTrait,
        text_input::TextInputManagerState,
        viewporter::ViewporterState,
        virtual_keyboard::VirtualKeyboardManagerState,
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
    xwayland::{xwm, X11Surface, X11Wm, XWayland, XWaylandEvent, XwmHandler},
};
use uuid::Uuid;

use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use dway_util::stat::PerfLog;

use crate::{
    math::{
        point_to_ivec2, point_to_vec2, rectangle_i32_to_rect, rectangle_to_rect, vec2_to_point,
    },
    wayland::{
        render::render_surface,
        surface::{
            ensure_initial_configure, print_surface_tree, try_get_component_locked,
            try_with_states_locked,
        },
    },
};

use self::{
    cursor::Cursor,
    focus::FocusTarget,
    grabs::MoveSurfaceGrab,
    render::{DummyRenderer, DummyTexture},
    shell::{place_new_window, ResizeState, WindowElement},
    surface::{get_component_locked, with_states_locked, DWaySurfaceData},
};

#[derive(Debug, Default)]
pub struct ClientState;
impl ClientData for ClientState {
    /// Notification that a client was initialized
    fn initialized(&self, _client_id: ClientId) {}
    /// Notification that a client is disconnected
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
pub struct CalloopData {
    pub state: DWayState,
    pub display: Display<DWayState>,
}
#[derive(Debug)]
pub struct DWayState {
    pub element_map: HashMap<Uuid, WindowElement>,
    pub surface_map: HashMap<Uuid, WlSurface>,
    pub x11_window_map: HashMap<u32, X11Surface>,
    pub outputs: Vec<Output>,
    pub tick: usize,

    pub display_number: Option<u32>,
    pub socket_name: String,
    pub display_handle: DisplayHandle,
    pub running: Arc<AtomicBool>,
    pub handle: LoopHandle<'static, CalloopData>,

    // desktop
    // pub space: Space<WindowElement>,
    pub popups: PopupManager,

    pub render: DummyRenderer,
    // smithay state
    pub compositor_state: CompositorState,
    pub data_device_state: DataDeviceState,
    pub layer_shell_state: WlrLayerShellState,
    pub output_manager_state: OutputManagerState,
    pub primary_selection_state: PrimarySelectionState,
    pub seat_state: SeatState<DWayState>,
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub shm_state: ShmState,
    pub viewporter_state: ViewporterState,
    pub xdg_activation_state: XdgActivationState,
    pub xdg_decoration_state: XdgDecorationState,
    pub xdg_shell_state: XdgShellState,
    pub presentation_state: PresentationState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,

    pub dnd_icon: Option<WlSurface>,
    pub log: slog::Logger,
    pub receiver: Receiver<WindowMessage>,
    pub sender: Sender<WindowMessage>,
    pub output: Output,
    pub damage_render: DamageTrackedRenderer,

    // input-related fields
    pub suppressed_keys: Vec<u32>,
    pub pointer_location: Point<f64, Logical>,
    pub cursor_status: Arc<Mutex<CursorImageStatus>>,
    pub seat_name: String,
    pub seat: Seat<DWayState>,
    pub clock: Clock<Monotonic>,

    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
}
impl DWayState {
    pub fn init(
        display: &mut Display<DWayState>,
        handle: LoopHandle<'static, CalloopData>,
        log: slog::Logger,
        receiver: Receiver<WindowMessage>,
        sender: Sender<WindowMessage>,
    ) -> Self {
        let clock = Clock::new().expect("failed to initialize clock");

        let size = (1920, 1080);
        let output = Output::new(
            "output".to_string(),
            PhysicalProperties {
                size: size.into(),
                subpixel: Subpixel::Unknown,
                make: "output".into(),
                model: "output".into(),
            },
            log.clone(),
        );
        let _global = output.create_global::<DWayState>(&display.handle());
        let mode = Mode {
            size: size.into(),
            refresh: 60_000,
        };
        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        let damage_render = DamageTrackedRenderer::from_output(&output);

        // init wayland clients
        let source = ListeningSocketSource::new_auto(log.clone()).unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();
        handle
            .insert_source(source, |client_stream, _, data| {
                if let Err(err) = data
                    .display
                    .handle()
                    .insert_client(client_stream, Arc::new(ClientState))
                {
                    slog::warn!(data.state.log, "Error adding wayland client: {}", err);
                };
            })
            .expect("Failed to init wayland socket source");
        info!(log, "Listening on wayland socket"; "name" => socket_name.clone());
        handle
            .insert_source(
                Generic::new(
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ,
                    smithay::reexports::calloop::Mode::Level,
                ),
                |_, _, data| {
                    data.display.dispatch_clients(&mut data.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .expect("Failed to init wayland server source");

        // init globals
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self, _>(&dh, log.clone());
        let data_device_state = DataDeviceState::new::<Self, _>(&dh, log.clone());
        let layer_shell_state = WlrLayerShellState::new::<Self, _>(&dh, log.clone());
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self, _>(&dh, log.clone());
        let mut seat_state = SeatState::new();
        let shm_state = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let viewporter_state = ViewporterState::new::<Self, _>(&dh, log.clone());
        let xdg_activation_state = XdgActivationState::new::<Self, _>(&dh, log.clone());
        let xdg_decoration_state = XdgDecorationState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state = XdgShellState::new::<Self, _>(&dh, log.clone());
        let presentation_state = PresentationState::new::<Self>(&dh, clock.id() as u32);
        let fractional_scale_manager_state =
            FractionalScaleManagerState::new::<Self, _>(&dh, log.clone());
        TextInputManagerState::new::<Self>(&dh);
        InputMethodManagerState::new::<Self>(&dh);
        VirtualKeyboardManagerState::new::<Self, _>(&dh, |_client| true);

        let render = DummyRenderer::default();

        // init input
        let seat_name = "dway".to_owned();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name.clone(), log.clone());

        let cursor_status = Arc::new(Mutex::new(CursorImageStatus::Default));
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to initialize the keyboard");

        let cursor_status2 = cursor_status.clone();
        seat.tablet_seat()
            .on_cursor_surface(move |_tool, new_status| {
                // TODO: tablet tools should have their own cursors
                *cursor_status2.lock().unwrap() = new_status;
            });

        seat.add_input_method(XkbConfig::default(), 200, 25);

        let dh = display.handle();
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);

        let xwayland = {
            let (xwayland, channel) = XWayland::new(log.clone(), &dh);
            let log2 = log.clone();
            let ret = handle.insert_source(channel, move |event, _, data| match event {
                XWaylandEvent::Ready {
                    connection,
                    client,
                    client_fd: _,
                    display,
                } => {
                    info!(log2, "xwayland ready");
                    data.state.display_number = Some(display);
                    let mut wm = X11Wm::start_wm(
                        data.state.handle.clone(),
                        dh.clone(),
                        connection,
                        client,
                        log2.clone(),
                    )
                    .expect("Failed to attach X11 Window Manager");
                    let cursor = Cursor::load(&log2);
                    let image = cursor.get_image(1, Duration::ZERO);
                    wm.set_cursor(
                        &image.pixels_rgba,
                        Size::from((image.width as u16, image.height as u16)),
                        Point::from((image.xhot as u16, image.yhot as u16)),
                    )
                    .expect("Failed to set xwayland default cursor");
                    data.state.xwm = Some(wm);
                }
                XWaylandEvent::Exited => {
                    warn!(log2, "xwayland exited");
                    let _ = data.state.xwm.take();
                }
            });
            if let Err(e) = ret {
                error!(
                    log,
                    "Failed to insert the XWaylandSource into the event loop: {}", e
                );
            }
            xwayland
        };
        let xwayland_env: [(OsString, OsString); 0] = [];
        if let Err(e) = xwayland.start(handle.clone(), None, xwayland_env, |_| {}) {
            error!(log, "Failed to start XWayland: {}", e);
        }

        DWayState {
            tick: 0,
            display_handle: display.handle(),
            display_number: None,
            socket_name,
            running: Arc::new(AtomicBool::new(true)),
            handle,
            // space: Space::new(log.clone()),
            popups: PopupManager::new(log.clone()),
            compositor_state,
            data_device_state,
            layer_shell_state,
            output_manager_state,
            primary_selection_state,
            seat_state,
            keyboard_shortcuts_inhibit_state,
            shm_state,
            viewporter_state,
            xdg_activation_state,
            xdg_decoration_state,
            xdg_shell_state,
            presentation_state,
            fractional_scale_manager_state,
            dnd_icon: None,
            log,
            suppressed_keys: Vec::new(),
            pointer_location: (0.0, 0.0).into(),
            cursor_status,
            seat_name,
            seat,
            clock,
            xwayland,
            xwm: None,
            receiver,
            sender,
            render,
            output,
            damage_render,
            surface_map: Default::default(),
            element_map: Default::default(),
            x11_window_map: Default::default(),
            outputs: Default::default(),
        }
    }

    pub fn element_for_uuid(&self, uuid: &Uuid) -> Option<&WindowElement> {
        self.element_map.get(uuid)
        // self.element_map.values()
        //     .find(|window| {
        //         if let Some(surface) = window.wl_surface() {
        //             with_states_locked(&surface, |s: &mut DWaySurfaceData| &s.uuid == uuid)
        //         } else {
        //             false
        //         }
        //     })
    }
    pub fn surface_for_uuid(&self, uuid: &Uuid) -> Option<&WlSurface> {
        self.surface_map.get(uuid)
    }
    // pub fn element_for_x11_surface(&self, surface: &X11Surface) -> Option<WindowElement> {
    //     self.x11_window_map.get(k)
    //     self.space
    //         .elements()
    //         .find(|window| match window {
    //             WindowElement::Wayland(_) => false,
    //             WindowElement::X11(w) => w == surface,
    //         })
    //         .cloned()
    // }
    pub fn element_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.element_map
            .values()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .or_else(|| {
                if let Some(popup_kind) = self.popups.find_popup(surface) {
                    match popup_kind {
                        PopupKind::Xdg(popup_surface) => {
                            if let Some(parent) = popup_surface.get_parent_surface() {
                                self.element_for_surface(&parent)
                            } else {
                                None
                            }
                        }
                    }
                } else {
                    None
                }
            })
    }
    pub fn element_and_geo_for_surface(
        &self,
        surface: &WlSurface,
    ) -> Option<(
        WindowElement,
        Rectangle<i32, Logical>,
        Rectangle<i32, Logical>,
    )> {
        self.element_map
            .values()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
            .and_then(|element| {
                // let s=DWaySurfaceData::get_physical_geometry_bbox(&element);
                if let Some((geo, bbox)) = DWaySurfaceData::get_logical_geometry_bbox(&element) {
                    Some((element, geo, bbox))
                } else {
                    None
                }
            })
            .or_else(|| {
                self.popups.find_popup(surface).and_then(|popup_kind| {
                    let popup_geometry = popup_kind.geometry();
                    let loc =
                        with_states_locked(surface, |s: &mut XdgPopupSurfaceRoleAttributes| {
                            s.current.geometry
                        });
                    match popup_kind {
                        PopupKind::Xdg(popup_surface) => {
                            popup_surface.get_parent_surface().and_then(|parent| {
                                self.element_and_geo_for_surface(&parent).map(
                                    |(element, geo, bbox)| {
                                        (
                                            element,
                                            Rectangle::from_loc_and_size(
                                                geo.loc + loc.loc + popup_geometry.loc,
                                                loc.size,
                                            ),
                                            Rectangle::from_loc_and_size(
                                                bbox.loc + loc.loc + popup_geometry.loc,
                                                loc.size,
                                            ),
                                        )
                                    },
                                )
                            })
                        }
                    }
                })
            })
    }

    pub fn spawn(&self, mut command: Command) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        command
            .env("WAYLAND_DISPLAY", self.socket_name.clone())
            .spawn()
            .unwrap();
    }

    pub fn send(&mut self, message: WindowMessage) {
        if let Err(e) = self.sender.send(message) {}
    }

    pub fn tick(&mut self) {
        self.tick += 1;
    }

    pub fn reset_buffers(&self) -> Fallible<()> {
        todo!()
    }

    pub fn debug(&self) {
        // info!(self.log, "element count {}", self.space.elements().count());
        // self.space.elements().for_each(|e| {
        //     if let Some(surface) = e.wl_surface() {
        //         print!("element {} ", e.id());
        //         print_surface_tree(&surface);
        //     } else {
        //         println!("element {} ", e.id());
        //     }
        // });
        // dbg!(&self.popups);
        // dbg!(&self.popups);
        // dbg!(&self.space);
    }
}
impl SeatHandler for DWayState {
    type KeyboardFocus = FocusTarget;

    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&Self::KeyboardFocus>) {}

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}
}
delegate_compositor!(DWayState);
impl CompositorHandler for DWayState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        X11Wm::commit_hook::<CalloopData>(surface);

        on_commit_buffer_handler(surface);

        let element = self.element_and_geo_for_surface(surface);
        if !is_sync_subsurface(surface) {
            if let Some((WindowElement::Wayland(window), ..)) = &element {
                window.on_commit();
            }
        }
        self.popups.commit(surface);

        ensure_initial_configure(self, surface);

        let scale = Scale { x: 1, y: 1 };
        if let Some((element, geo, bbox)) = element {
            let geo = geo.to_physical_precise_round(scale);
            let bbox = bbox.to_physical_precise_round(scale);
            if let Err(e) = render_surface(self, surface, geo, bbox) {
                error!(self.log, "{e}");
            }
        } else {
            warn!(self.log, "surface source not found: {:?}", surface.id());
        }
        trace!(self.log, "commited {:?}", surface.id());
    }
}
delegate_data_device!(DWayState);
impl ClientDndGrabHandler for DWayState {
    fn started(
        &mut self,
        source: Option<smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource>,
        icon: Option<WlSurface>,
        seat: Seat<Self>,
    ) {
        info!(self.log, "ClientDndGrabHandler::started");
    }

    fn dropped(&mut self, seat: Seat<Self>) {
        info!(self.log, "ClientDndGrabHandler::started");
    }
}
impl ServerDndGrabHandler for DWayState {
    fn action(
        &mut self,
        action: smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction,
    ) {
        info!(self.log, "ServerDndGrabHandler::action");
    }

    fn dropped(&mut self) {
        info!(self.log, "ServerDndGrabHandler::dropped");
    }

    fn cancelled(&mut self) {
        info!(self.log, "ServerDndGrabHandler::cancelled");
    }

    fn send(&mut self, mime_type: String, fd: OwnedFd) {
        info!(self.log, "ServerDndGrabHandler::send");
    }

    fn finished(&mut self) {
        info!(self.log, "ServerDndGrabHandler::finished");
    }
}
impl DataDeviceHandler for DWayState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }

    fn action_choice(
        &mut self,
        available: smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction,
        preferred: smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction,
    ) -> smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction {
        smithay::wayland::data_device::default_action_chooser(available, preferred)
    }

    fn new_selection(
        &mut self,
        source: Option<smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource>,
    ) {
    }

    fn send_selection(&mut self, mime_type: String, fd: OwnedFd) {}
}
delegate_output!(DWayState);
delegate_primary_selection!(DWayState);
impl PrimarySelectionHandler for DWayState {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }

    fn new_selection(
        &mut self,
        source: Option<smithay::reexports::wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1>,
    ) {
    }

    fn send_selection(&mut self, mime_type: String, fd: OwnedFd) {}
}
delegate_shm!(DWayState);
impl BufferHandler for DWayState {
    fn buffer_destroyed(
        &mut self,
        buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
        trace!(self.log,"<DWayState as BufferHandler>::buffer_destroyed";"id"=>buffer.id().to_string());
    }
}
impl ShmHandler for DWayState {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
delegate_seat!(DWayState);
delegate_tablet_manager!(DWayState);
delegate_text_input_manager!(DWayState);
delegate_input_method_manager!(DWayState);
delegate_keyboard_shortcuts_inhibit!(DWayState);
impl KeyboardShortcutsInhibitHandler for DWayState {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.keyboard_shortcuts_inhibit_state
    }

    fn new_inhibitor(
        &mut self,
        inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
    ) {
    }

    fn inhibitor_destroyed(
        &mut self,
        inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
    ) {
    }
}
delegate_virtual_keyboard_manager!(DWayState);
delegate_viewporter!(DWayState);
delegate_xdg_activation!(DWayState);
impl XdgActivationHandler for DWayState {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        todo!()
    }

    fn destroy_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        todo!()
    }
}
delegate_xdg_decoration!(DWayState);
impl XdgDecorationHandler for DWayState {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        debug!(self.log, "new_decoration");
        toplevel.with_pending_state(|state| {
            state.decoration_mode =
                Some(xdg_decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ClientSide);
        });
        toplevel.send_configure();
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: DecorationMode) {
        todo!()
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        debug!(self.log, "unset_mode");
        toplevel.with_pending_state(|state| {
            state.decoration_mode =
                Some(xdg_decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ClientSide);
        });
        let initial_configure_sent = with_states_locked(
            toplevel.wl_surface(),
            |s: &mut XdgToplevelSurfaceRoleAttributes| s.initial_configure_sent,
        );
        if initial_configure_sent {
            toplevel.send_configure();
        }
    }
}
impl XdgShellHandler for DWayState {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        debug!(self.log, "new_toplevel");
        // let rect = place_new_window(&mut self.space, &window, true);
        let rect = Rectangle::<i32, Logical>::from_loc_and_size((75, 75), (800, 600));
        with_surfaces_surface_tree(surface.wl_surface(), |s, states| {
            states.data_map.insert_if_missing(|| {
                let uuid = Uuid::new_v4();
                if s == surface.wl_surface() {
                    self.element_map
                        .insert(uuid, WindowElement::Wayland(Window::new(surface.clone())));
                }
                self.surface_map.insert(uuid, s.clone());
                self.send(WindowMessage {
                    uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Create {
                        pos: Vec2::new(rect.loc.x as f32, rect.loc.y as f32),
                        size: Vec2::new(rect.size.w as f32, rect.size.h as f32),
                    },
                });
                let mut data = DWaySurfaceData::new(uuid);
                data.geo = rect;
                data.bbox = rect;
                Mutex::new(data)
            });
        });
    }

    fn new_popup(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        positioner: smithay::wayland::shell::xdg::PositionerState,
    ) {
        debug!(self.log, "new_popup");
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
        });
        let rect = positioner.get_geometry();
        let parent_geo = surface
            .get_parent_surface()
            .and_then(|surface| self.element_for_surface(&surface))
            .as_ref()
            .and_then(|element| DWaySurfaceData::get_logical_geometry_bbox(element))
            .map(|(geo, _)| geo)
            .unwrap_or_default();
        with_surfaces_surface_tree(surface.wl_surface(), |surface, states| {
            states.data_map.insert_if_missing(|| {
                let uuid = Uuid::new_v4();
                self.surface_map.insert(uuid, surface.clone());
                let geo = Rectangle::from_loc_and_size(rect.loc + parent_geo.loc, rect.size);
                self.send(WindowMessage {
                    uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Create {
                        pos: point_to_vec2(geo.loc.to_f64()),
                        size: point_to_vec2(geo.size.to_f64().to_point()),
                    },
                });
                Mutex::new(DWaySurfaceData::new(uuid))
            });
        });
        if let Err(err) = self.popups.track_popup(PopupKind::from(surface)) {
            slog::warn!(self.log, "Failed to track popup: {}", err);
        }
    }

    fn grab(
        &mut self,
        surface: smithay::wayland::shell::xdg::PopupSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        debug!(self.log, "grab");
        let seat: Seat<Self> = Seat::from_resource(&seat).unwrap();
        let kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&kind).ok().and_then(|root| {
            self.element_map
                .values()
                .find(|w| w.wl_surface().map(|s| s == root).unwrap_or(false))
                .cloned()
                .map(FocusTarget::Window)
                .or_else(|| {
                    self.outputs
                        .iter()
                        .find_map(|o| {
                            let map = layer_map_for_output(o);
                            map.layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                                .cloned()
                        })
                        .map(FocusTarget::LayerSurface)
                })
        }) {
            let ret = self.popups.grab_popup(root, kind, &seat, serial);

            if let Ok(mut grab) = ret {
                if let Some(keyboard) = seat.get_keyboard() {
                    if keyboard.is_grabbed()
                        && !(keyboard.has_grab(serial)
                            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    keyboard.set_focus(self, grab.current_grab(), serial);
                    keyboard.set_grab(PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial)
                            || pointer
                                .has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
        }
    }

    fn new_client(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn client_pong(&mut self, client: smithay::wayland::shell::xdg::ShellClient) {}

    fn move_request(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
    ) {
        let uuid = with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid);
        self.send(WindowMessage {
            uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::MoveRequest,
        });
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
        let uuid = with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid);
        let (top, bottom, left, right) = match edges {
            xdg_toplevel::ResizeEdge::None => (false, false, false, false),
            xdg_toplevel::ResizeEdge::Top => (true, false, false, false),
            xdg_toplevel::ResizeEdge::Bottom => (false, true, false, false),
            xdg_toplevel::ResizeEdge::Left => (false, false, true, false),
            xdg_toplevel::ResizeEdge::TopLeft => (true, false, true, false),
            xdg_toplevel::ResizeEdge::BottomLeft => (false, true, true, false),
            xdg_toplevel::ResizeEdge::Right => (false, false, false, true),
            xdg_toplevel::ResizeEdge::TopRight => (true, false, false, true),
            xdg_toplevel::ResizeEdge::BottomRight => (false, true, false, true),
            _ => return,
        };
        info!(self.log, "resizing request");
        self.send(WindowMessage {
            uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::ResizeRequest {
                top,
                bottom,
                left,
                right,
            },
        });
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::Maximized,
        });
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        debug!(self.log, "unmaximize_request");
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::Unmaximized,
        });
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::FullScreen,
        });
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::UnFullScreen,
        });
    }

    fn minimize_request(&mut self, surface: ToplevelSurface) {
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::Minimized,
        });
    }

    fn show_window_menu(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        location: Point<i32, Logical>,
    ) {
        debug!(self.log, "show_window_menu");
        // let uuid = Uuid::new_v4();
        // with_surface_tree_upward(
        //     surface.wl_surface(),
        //     (),
        //     |_, _, _| TraversalAction::DoChildren(()),
        //     |_, states, _| {
        //         states.data_map.insert_if_missing(|| {
        //             RefCell::new(Mutex {
        //                 uuid,
        //                 ..Default::default()
        //             })
        //         });
        //     },
        //     |_, _, _| true,
        // );
        // let rect = Rectangle::from_loc_and_size(location,(200,300));
        // self
        //     .send(WindowMessage {
        //         uuid,
        //         time: SystemTime::now(),
        //         data: WindowMessageKind::Create {
        //             pos: Vec2::new(rect.loc.x as f32, rect.loc.y as f32),
        //             size: Vec2::new(rect.size.w as f32, rect.size.h as f32),
        //         },
        //     })
        //     ;
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
        surface.with_pending_state(|state| {
            let geometry = positioner.get_geometry();
            state.geometry = geometry;
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::SetRect(rectangle_to_rect(positioner.get_geometry().to_f64())),
        });
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        with_surfaces_surface_tree(surface.wl_surface(), |surface, states| {
            if let Some(surface_data) = try_get_component_locked::<DWaySurfaceData>(states) {
                self.surface_map.remove(&surface_data.uuid);
            }
        });
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::Destroy,
        });
    }

    fn popup_destroyed(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface) {
        with_surfaces_surface_tree(surface.wl_surface(), |surface, states| {
            let surface_data = get_component_locked::<DWaySurfaceData>(states);
            self.surface_map.remove(&surface_data.uuid);
        });
        self.send(WindowMessage {
            uuid: with_states_locked(surface.wl_surface(), |s: &mut DWaySurfaceData| s.uuid),
            time: SystemTime::now(),
            data: WindowMessageKind::Destroy,
        });
    }
}
delegate_xdg_shell!(DWayState);
impl WlrLayerShellHandler for DWayState {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
        layer: smithay::wayland::shell::wlr_layer::Layer,
        namespace: String,
    ) {
        todo!()
    }

    fn new_popup(
        &mut self,
        parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        popup: smithay::wayland::shell::xdg::PopupSurface,
    ) {
        todo!()
    }

    fn ack_configure(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure,
    ) {
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {}
}
delegate_layer_shell!(DWayState);
delegate_presentation!(DWayState);
impl FractionScaleHandler for DWayState {
    fn new_fractional_scale(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        info!(self.log,"new_fractional_scale";"surface"=>surface.id().to_string());
    }
}
delegate_fractional_scale!(DWayState);

impl XwmHandler for CalloopData {
    fn xwm_state(&mut self, xwm: smithay::xwayland::xwm::XwmId) -> &mut X11Wm {
        self.state.xwm.as_mut().unwrap()
    }

    fn new_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        let uuid = Uuid::new_v4();
        info!(
            self.state.log,
            "create x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        dbg!("new_window", window.window_id(), uuid);
        self.state
            .x11_window_map
            .insert(window.window_id(), window.clone());
        self.state
            .element_map
            .insert(uuid, WindowElement::X11(window));
    }

    fn new_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        let uuid = Uuid::new_v4();
        info!(
            self.state.log,
            "new_override_redirect_window x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        dbg!("new_window", window.window_id(), uuid);
        self.state
            .x11_window_map
            .insert(window.window_id(), window.clone());
        self.state
            .element_map
            .insert(uuid, WindowElement::X11(window));
    }

    fn map_window_notify(&mut self, xwm: xwm::XwmId, window: X11Surface) {
        let Some(surface)=window.wl_surface()else{
            error!(self.state.log,"failed to get surface";"window"=>window.window_id());
            return;
        };
        info!(
            self.state.log,
            "map_window_notify {:?}:{:?} => {:?}",
            xwm,
            window.window_id(),
            surface.id()
        );
        DWaySurfaceData::update_x11_surface_geometry(&window);
        let rect = window.geometry();
        with_surfaces_surface_tree(&surface, |s, states| {
            states.data_map.insert_if_missing(|| {
                let uuid = Uuid::new_v4();
                if s == &surface {
                    self.state
                        .element_map
                        .insert(uuid, WindowElement::X11(window.clone()));
                }
                self.state.surface_map.insert(uuid, s.clone());
                self.state.send(WindowMessage {
                    uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Create {
                        pos: Vec2::new(rect.loc.x as f32, rect.loc.y as f32),
                        size: Vec2::new(rect.size.w as f32, rect.size.h as f32),
                    },
                });
                let mut data = DWaySurfaceData::new(uuid);
                data.geo = rect;
                data.bbox = rect;
                Mutex::new(data)
            });
        });
    }

    fn map_window_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        info!(
            self.state.log,
            "map_window_request x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        window.set_mapped(true).unwrap();
        DWaySurfaceData::update_x11_surface_geometry(&window);
    }

    fn mapped_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        info!(
            self.state.log,
            "mapped_override_redirect_window x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        dbg!("mapped_override_redirect_window", window.geometry());
        DWaySurfaceData::update_x11_surface_geometry(&window);
    }

    fn unmapped_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        info!(
            self.state.log,
            "unmapped_window x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        self.state.x11_window_map.remove(&window.window_id());
        let Some(surface) = window.wl_surface() else{
            error!(self.state.log,"surface of x11 window not found {:?}",window.window_id());
            return;
        };
        self.state.x11_window_map.remove(&window.window_id());
        with_surfaces_surface_tree(&surface, |surface, states| {
            if let Some(surface_data) = try_get_component_locked::<DWaySurfaceData>(states) {
                let uuid = surface_data.uuid;
                self.state.surface_map.remove(&uuid);
                self.state.send(WindowMessage {
                    uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Destroy,
                });
            } else {
                warn!(
                    self.state.log,
                    "uuid of x11 surface not found {}",
                    window.window_id()
                );
            }
        });
        if !window.is_override_redirect() {
            window.set_mapped(false).unwrap();
        }
    }

    fn destroyed_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        info!(
            self.state.log,
            "destroyed x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        self.state.x11_window_map.remove(&window.window_id());
        if let Some(surface) = window.wl_surface() {
            with_surfaces_surface_tree(&surface, |surface, states| {
                let status = get_component_locked::<DWaySurfaceData>(states);
                self.state.send(WindowMessage {
                    uuid: status.uuid,
                    time: SystemTime::now(),
                    data: WindowMessageKind::Destroy,
                });
            });
        }
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
        info!(
            self.state.log,
            "configure_request x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        let mut geo = window.geometry();
        if let Some(w) = w {
            geo.size.w = w as i32;
        }
        if let Some(h) = h {
            geo.size.h = h as i32;
        }
        if let Err(e) = window.configure(geo) {
            error!(self.state.log, "error while configure_request: {e:?}");
        };
    }

    fn configure_notify(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: Rectangle<i32, Logical>,
        above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        info!(
            self.state.log,
            "configure_request x11 window: {:?} {:?}",
            xwm,
            window.window_id()
        );
        DWaySurfaceData::update_x11_surface_geometry(&window);
    }

    fn maximize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::Maximized,
            });
        }
    }

    fn unmaximize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::Unmaximized,
            });
        }
    }

    fn fullscreen_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::FullScreen,
            });
        }
    }

    fn unfullscreen_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::UnFullScreen,
            });
        }
    }

    fn minimize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::Minimized,
            });
        }
    }

    fn unminimize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::Unminimized,
            });
        }
    }

    fn resize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        let (top, bottom, left, right) = match resize_edge {
            xwm::ResizeEdge::Top => (true, false, false, false),
            xwm::ResizeEdge::Bottom => (false, true, false, false),
            xwm::ResizeEdge::Left => (false, false, true, false),
            xwm::ResizeEdge::TopLeft => (true, false, true, false),
            xwm::ResizeEdge::BottomLeft => (false, true, true, false),
            xwm::ResizeEdge::Right => (false, false, false, true),
            xwm::ResizeEdge::TopRight => (true, false, false, true),
            xwm::ResizeEdge::BottomRight => (false, true, false, true),
            _ => return,
        };
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::ResizeRequest {
                    top,
                    bottom,
                    left,
                    right,
                },
            });
        }
    }

    fn move_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
    ) {
        if let Some(surface) = window.wl_surface() {
            let uuid = with_states_locked(&surface, |s: &mut DWaySurfaceData| s.uuid);
            self.state.send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::MoveRequest,
            });
        }
    }
}
