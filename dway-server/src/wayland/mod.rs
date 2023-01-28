pub mod cursor;
pub mod focus;
pub mod grabs;
pub mod inputs;
pub mod render;
pub mod shell;
pub mod surface;

use std::{
    any::TypeId,
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
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
        find_popup_root_surface, layer_map_for_output, space::SpaceElement, PopupKeyboardGrab,
        PopupKind, PopupManager, PopupPointerGrab, PopupUngrabStrategy, Space, Window,
        WindowSurfaceType,
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
    },
    utils::{Clock, Logical, Monotonic, Point, Rectangle, Size, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_surface_tree_upward, CompositorHandler,
            CompositorState, TraversalAction,
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
                ToplevelSurface, XdgShellHandler, XdgShellState, XdgToplevelSurfaceRoleAttributes,
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
    xwayland::{X11Wm, XWayland, XWaylandEvent, XwmHandler},
};
use uuid::Uuid;

use dway_protocol::window::{ImageBuffer, WindowMessage, WindowMessageKind};
use dway_util::stat::PerfLog;

use crate::wayland::{
    render::render_surface,
    surface::{ensure_initial_configure, with_states_borrowed},
};

use self::{
    cursor::Cursor,
    focus::FocusTarget,
    grabs::MoveSurfaceGrab,
    render::{DummyRenderer, DummyTexture},
    shell::{place_new_window, ResizeState, WindowElement},
    surface::{with_states_borrowed_mut, with_states_locked, SurfaceData},
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
impl XwmHandler for CalloopData {
    fn xwm_state(&mut self, xwm: smithay::xwayland::xwm::XwmId) -> &mut X11Wm {
        todo!()
    }

    fn new_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn new_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn map_window_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn mapped_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn unmapped_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn destroyed_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
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
        todo!()
    }

    fn configure_notify(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: smithay::utils::Rectangle<i32, Logical>,
        above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        todo!()
    }

    fn resize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        todo!()
    }

    fn move_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
    ) {
        todo!()
    }
}
pub struct DWayState {
    pub tick: usize,

    pub socket_name: String,
    pub display_handle: DisplayHandle,
    pub running: Arc<AtomicBool>,
    pub handle: LoopHandle<'static, CalloopData>,

    // desktop
    pub space: Space<WindowElement>,
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
                    display: _,
                } => {
                    let mut wm = X11Wm::start_wm(
                        data.state.handle.clone(),
                        dh.clone(),
                        connection,
                        client,
                        log2.clone(),
                    )
                    .expect("Failed to attach X11 Window Manager");
                    let cursor = Cursor::load();
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

        DWayState {
            tick: 0,
            display_handle: display.handle(),
            socket_name,
            running: Arc::new(AtomicBool::new(true)),
            handle,
            space: Space::new(log.clone()),
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
        }
    }

    pub fn window_for_uuid(&self, uuid: &Uuid) -> Option<WindowElement> {
        self.space
            .elements()
            .find(|window| {
                if let Some(surface) = window.wl_surface() {
                    with_states_borrowed(&surface, |s: &SurfaceData| &s.uuid == uuid)
                } else {
                    false
                }
            })
            .cloned()
    }
    pub fn window_for_surface(&self, surface: &WlSurface) -> Option<WindowElement> {
        self.space
            .elements()
            .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
            .cloned()
    }

    pub fn spawn(&self, mut command: Command) {
        command
            .env("WAYLAND_DISPLAY", self.socket_name.clone())
            .env_remove("DISPLAY")
            .spawn()
            .unwrap();
    }

    pub fn tick(&mut self) {
        self.tick += 1;
    }

    pub(crate) fn reset_buffers(&self) -> Fallible<()> {
        todo!()
    }
}
impl SeatHandler for DWayState {
    type KeyboardFocus = FocusTarget;

    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}
delegate_compositor!(DWayState);
impl CompositorHandler for DWayState {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        X11Wm::commit_hook::<Self>(surface);

        on_commit_buffer_handler(surface);

        let element = self.window_for_surface(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(WindowElement::Wayland(window)) = &element {
                window.on_commit();
            }
        }
        self.popups.commit(surface);

        ensure_initial_configure(surface, &self.space, &mut self.popups);

        if let Some(element) = element {
            if let Err(e) = render_surface(self, &element, surface) {
                error!(self.log, "{e}");
            }
        }
        trace!(self.log, "commited {:?}", surface.id());
    }
}
delegate_data_device!(DWayState);
impl ClientDndGrabHandler for DWayState {}
impl ServerDndGrabHandler for DWayState {}
impl DataDeviceHandler for DWayState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
delegate_output!(DWayState);
delegate_primary_selection!(DWayState);
impl PrimarySelectionHandler for DWayState {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
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
        let uuid = Uuid::new_v4();
        with_surface_tree_upward(
            surface.wl_surface(),
            (),
            |_, _, _| TraversalAction::DoChildren(()),
            |_, states, _| {
                states.data_map.insert_if_missing(|| {
                    RefCell::new(SurfaceData {
                        uuid,
                        ..Default::default()
                    })
                });
            },
            |_, _, _| true,
        );
        let window = WindowElement::Wayland(Window::new(surface));
        let rect = place_new_window(&mut self.space, &window, true);
        self.sender
            .send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::Create {
                    pos: Vec2::new(rect.loc.x as f32, rect.loc.y as f32),
                    size: Vec2::new(rect.size.w as f32, rect.size.h as f32),
                },
            })
            .unwrap();
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
            self.space
                .elements()
                .find(|w| w.wl_surface().map(|s| s == root).unwrap_or(false))
                .cloned()
                .map(FocusTarget::Window)
                .or_else(|| {
                    self.space
                        .outputs()
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
        let uuid = with_states_borrowed(surface.wl_surface(), |s: &SurfaceData| s.uuid);
        self.sender
            .send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::MoveRequest,
            })
            .unwrap();
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        edges: smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
    ) {
        let uuid = with_states_borrowed(surface.wl_surface(), |s: &SurfaceData| s.uuid);
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
        self.sender
            .send(WindowMessage {
                uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::ResizeRequest {
                    top,
                    bottom,
                    left,
                    right,
                },
            })
            .unwrap();
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {}

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {}

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
    ) {
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {}

    fn minimize_request(&mut self, surface: ToplevelSurface) {}

    fn show_window_menu(
        &mut self,
        surface: ToplevelSurface,
        seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        serial: smithay::utils::Serial,
        location: Point<i32, Logical>,
    ) {
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
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {}

    fn popup_destroyed(&mut self, surface: smithay::wayland::shell::xdg::PopupSurface) {}
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
}
delegate_layer_shell!(DWayState);
delegate_presentation!(DWayState);
impl FractionScaleHandler for DWayState {
    fn new_fractional_scale(
        &mut self,
        surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        todo!()
    }
}
delegate_fractional_scale!(DWayState);

impl XwmHandler for DWayState {
    fn xwm_state(&mut self, xwm: smithay::xwayland::xwm::XwmId) -> &mut X11Wm {
        todo!()
    }

    fn new_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn new_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn map_window_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn mapped_override_redirect_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn unmapped_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
    }

    fn destroyed_window(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
    ) {
        todo!()
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
        todo!()
    }

    fn configure_notify(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        geometry: Rectangle<i32, Logical>,
        above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
    ) {
        todo!()
    }

    fn resize_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
        resize_edge: smithay::xwayland::xwm::ResizeEdge,
    ) {
        todo!()
    }

    fn move_request(
        &mut self,
        xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        button: u32,
    ) {
        todo!()
    }
}
