pub mod backend;
pub mod cursor;
pub mod ecs;
pub mod focus;
pub mod grabs;
pub mod inputs;
pub mod render;
pub mod shell;
pub mod surface;
pub mod x11;

use std::sync::atomic::AtomicBool;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::OsString,
    os::{
        fd::FromRawFd,
        unix::{
            io::{AsRawFd, OwnedFd},
            raw::dev_t,
        },
    },
    path::PathBuf,
    process::Command,
    rc::Rc,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use bevy_math::Vec2;
use crossbeam_channel::{Receiver, Sender};
use failure::Fallible;
use slog::{debug, error, info, trace, warn};
use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        drm::{DrmDevice, DrmDeviceFd, DrmError, DrmEvent, DrmEventMetadata, DrmNode},
        renderer::{
            damage::DamageTrackedRenderer, element::texture::TextureBuffer,
            gles2::Gles2Renderbuffer, utils::on_commit_buffer_handler, ImportDma,
        },
        session::Session,
        SwapBuffersError,
    },
    delegate_compositor, delegate_data_device, delegate_dmabuf, delegate_fractional_scale,
    delegate_input_method_manager, delegate_keyboard_shortcuts_inhibit, delegate_layer_shell,
    delegate_output, delegate_presentation, delegate_primary_selection, delegate_seat,
    delegate_shm, delegate_tablet_manager, delegate_text_input_manager, delegate_viewporter,
    delegate_virtual_keyboard_manager, delegate_xdg_activation, delegate_xdg_decoration,
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, utils::with_surfaces_surface_tree,
        PopupKeyboardGrab, PopupKind, PopupManager, PopupPointerGrab, PopupUngrabStrategy, Space,
        Window, WindowSurfaceType,
    },
    input::{
        keyboard::XkbConfig,
        pointer::{CursorImageStatus, Focus},
        Seat, SeatHandler, SeatState,
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            generic::Generic,
            timer::{TimeoutAction, Timer},
            Dispatcher, Interest, LoopHandle, PostAction,
        },
        drm::{self, control::crtc},
        gbm,
        nix::fcntl::OFlag,
        wayland_protocols::{
            wp::presentation_time::server::wp_presentation_feedback,
            xdg::{
                decoration::{
                    self as xdg_decoration,
                    zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
                },
                shell::server::xdg_toplevel,
            },
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle, Resource,
        },
    },
    utils::{Clock, DeviceFd, Logical, Monotonic, Point, Rectangle, Scale, Size, Transform},
    wayland::{
        buffer::BufferHandler,
        compositor::{is_sync_subsurface, CompositorHandler, CompositorState},
        data_device::{
            ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
        },
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
        fractional_scale::{FractionScaleHandler, FractionalScaleManagerState},
        input_method::{InputMethodManagerState, InputMethodSeat},
        keyboard_shortcuts_inhibit::{
            KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
        },
        output::OutputManagerState,
        presentation::PresentationState,
        primary_selection::{PrimarySelectionHandler, PrimarySelectionState},
        shell::{
            wlr_layer::{WlrLayerShellHandler, WlrLayerShellState},
            xdg::{
                decoration::{XdgDecorationHandler, XdgDecorationState},
                ToplevelSurface, XdgPopupSurfaceRoleAttributes, XdgShellHandler, XdgShellState,
                XdgToplevelSurfaceRoleAttributes,
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

use dway_protocol::window::{WindowMessage, WindowMessageKind};

use crate::{
    math::{point_to_vec2, rectangle_to_rect},
    wayland::{
        backend::udev::schedule_initial_render,
        render::render_surface,
        shell::place_new_window,
        surface::{ensure_initial_configure, try_get_component_locked},
    },
};

use self::inputs::process_input_event;
use self::{
    backend::{
        udev::{scan_connectors, BackendData, UDevBackend, UdevOutputId},
        Backend,
    },
    cursor::Cursor,
    focus::FocusTarget,
    render::DummyRenderer,
    shell::{fixup_positions, WindowElement},
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

    pub backend: Backend,

    pub display_number: Option<u32>,
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
    pub seat: Seat<DWayState>,
    pub clock: Clock<Monotonic>,

    pub xwayland: XWayland,
    pub xwm: Option<X11Wm>,
}
impl DWayState {
    pub fn init(
        display: &mut Display<DWayState>,
        handle: LoopHandle<'static, CalloopData>,
        backend: Backend,
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
        let source = ListeningSocketSource::new_auto().unwrap();
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
        let compositor_state = CompositorState::new::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh, );
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self>(&dh, );
        let mut seat_state = SeatState::new();
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let viewporter_state = ViewporterState::new::<Self>(&dh);
        let xdg_activation_state = XdgActivationState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let presentation_state = PresentationState::new::<Self>(&dh, clock.id() as u32);
        let fractional_scale_manager_state =
            FractionalScaleManagerState::new::<Self>(&dh);
        TextInputManagerState::new::<Self>(&dh);
        InputMethodManagerState::new::<Self>(&dh);
        VirtualKeyboardManagerState::new::<Self, _>(&dh, |_client| true);

        let render = DummyRenderer::default();

        // init input
        let seat_name = backend.seat_name();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name.clone());

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
            let (xwayland, channel) = XWayland::new( &dh);
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
            space: Space::new(log.clone()),
            backend,
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
    pub fn udev_backend(&self) -> &UDevBackend {
        match &self.backend {
            Backend::UDev(u) => u,
            Backend::Winit(_) => panic!(),
            Backend::Headless => panic!(),
        }
    }
    pub fn udev_backend_mut(&mut self) -> &mut UDevBackend {
        match &mut self.backend {
            Backend::UDev(u) => u,
            Backend::Winit(_) => panic!(),
            Backend::Headless => panic!(),
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
        if let Err(_e) = self.sender.send(message) {}
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

    fn device_added(&mut self, display: &mut Display<Self>, device_id: dev_t, path: PathBuf) {
        warn!(self.log, "add device {:?} {:?}", device_id, path);
        // Try to open the device
        let open_flags = OFlag::O_RDWR | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_NONBLOCK;
        let device_fd = self.udev_backend_mut().session.open(&path, open_flags).ok();
        let devices = device_fd
            .map(|fd| DrmDeviceFd::new(unsafe { DeviceFd::from_raw_fd(fd) }))
            .map(|fd| {
                (
                    DrmDevice::new(fd.clone(), true),
                    gbm::Device::new(fd),
                )
            });

        // Report device open failures.
        let (device, gbm) = match devices {
            Some((Ok(drm), Ok(gbm))) => (drm, gbm),
            Some((Err(err), _)) => {
                warn!(
                    self.log,
                    "Skipping device {:?}, because of drm error: {}", device_id, err
                );
                return;
            }
            Some((_, Err(err))) => {
                // TODO try DumbBuffer allocator in this case
                warn!(
                    self.log,
                    "Skipping device {:?}, because of gbm error: {}", device_id, err
                );
                return;
            }
            None => return,
        };

        let node = match DrmNode::from_dev_id(device_id) {
            Ok(node) => node,
            Err(err) => {
                warn!(
                    self.log,
                    "Failed to access drm node for {}: {}", device_id, err
                );
                return;
            }
        };
        let backends = Rc::new(RefCell::new(scan_connectors(
            node,
            &device,
            &gbm,
            display,
            &mut self.space,
            &self.log,
        )));

        let event_dispatcher = Dispatcher::new(
            device,
            move |event, metadata, data: &mut CalloopData| match event {
                DrmEvent::VBlank(crtc) => {
                    data.state.frame_finish(node, crtc, metadata);
                }
                DrmEvent::Error(error) => {
                    error!(data.state.log, "{:?}", error);
                }
            },
        );
        let registration_token = self
            .handle
            .register_dispatcher(event_dispatcher.clone())
            .unwrap();

        for backend in backends.borrow_mut().values() {
            // render first frame
            trace!(self.log, "Scheduling frame");
            let handle = &self.handle.clone();
            let log = self.log.clone();
            schedule_initial_render(
                &mut self.udev_backend_mut().gpus,
                backend.clone(),
                handle,
                log,
            );
        }

        self.udev_backend_mut().backends.insert(
            node,
            BackendData {
                registration_token,
                event_dispatcher,
                surfaces: backends,
                gbm,
            },
        );
    }

    fn device_changed(&mut self, display: &mut Display<Self>, device: dev_t) {
        let udev = {
            match &mut self.backend {
                Backend::UDev(u) => u,
                Backend::Winit(_) => panic!(),
                Backend::Headless => panic!(),
            }
        };
        let node = match DrmNode::from_dev_id(device).ok() {
            Some(node) => node,
            None => return, // we already logged a warning on device_added
        };
        let logger = self.log.clone();
        let loop_handle = self.handle.clone();

        //quick and dirty, just re-init all backends
        if let Some(ref mut backend_data) = udev.backends.get_mut(&node) {
            // scan_connectors will recreate the outputs (and sadly also reset the scales)
            for output in self
                .space
                .outputs()
                .filter(|o| {
                    o.user_data()
                        .get::<UdevOutputId>()
                        .map(|id| id.device_id == node)
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
            {
                self.space.unmap_output(&output);
            }

            let source = backend_data.event_dispatcher.as_source_mut();
            let mut backends = backend_data.surfaces.borrow_mut();
            *backends = scan_connectors(
                node,
                &source,
                &backend_data.gbm,
                display,
                &mut self.space,
                &logger,
            );

            // fixup window coordinates
            fixup_positions(&mut self.space);

            for surface in backends.values() {
                let logger = logger.clone();
                // render first frame
                schedule_initial_render(&mut udev.gpus, surface.clone(), &loop_handle, logger);
            }
        }
    }

    fn device_removed(&mut self, device: dev_t) {
        let node = match DrmNode::from_dev_id(device).ok() {
            Some(node) => node,
            None => return, // we already logged a warning on device_added
        };
        // drop the backends on this side
        if let Some(backend_data) = self.udev_backend_mut().backends.remove(&node) {
            // drop surfaces
            backend_data.surfaces.borrow_mut().clear();
            debug!(self.log, "Surfaces dropped");

            for output in self
                .space
                .outputs()
                .filter(|o| {
                    o.user_data()
                        .get::<UdevOutputId>()
                        .map(|id: &UdevOutputId| id.device_id == node)
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
            {
                self.space.unmap_output(&output);
            }
            fixup_positions(&mut self.space);

            self.handle.remove(backend_data.registration_token);
            let _device = backend_data.event_dispatcher.into_source_inner();

            debug!(self.log, "Dropping device");
        }
    }

    fn frame_finish(
        &mut self,
        dev_id: DrmNode,
        crtc: crtc::Handle,
        metadata: &mut Option<DrmEventMetadata>,
    ) {
        let log = self.log.clone();
        let device_backend = match self.udev_backend().backends.get(&dev_id) {
            Some(backend) => backend,
            None => {
                error!(
                    self.log,
                    "Trying to finish frame on non-existent backend {}", dev_id
                );
                return;
            }
        };

        let surfaces = device_backend.surfaces.borrow();
        let surface = match surfaces.get(&crtc) {
            Some(surface) => surface.clone(),
            None => {
                error!(
                    log,
                    "Trying to finish frame on non-existent crtc {:?}", crtc
                );
                return;
            }
        };

        let mut surface = surface.borrow_mut();

        let output = if let Some(output) = self.space.outputs().find(|o| {
            o.user_data().get::<UdevOutputId>()
                == Some(&UdevOutputId {
                    device_id: surface.device_id,
                    crtc,
                })
        }) {
            output.clone()
        } else {
            // somehow we got called with an invalid output
            return;
        };

        let schedule_render = match surface
            .surface
            .frame_submitted()
            .map_err(Into::<SwapBuffersError>::into)
        {
            Ok(user_data) => {
                if let Some(mut feedback) = user_data.flatten() {
                    let tp = metadata.as_ref().and_then(|metadata| match metadata.time {
                        smithay::backend::drm::DrmEventTime::Monotonic(tp) => Some(tp),
                        smithay::backend::drm::DrmEventTime::Realtime(_) => None,
                    });
                    let seq = metadata
                        .as_ref()
                        .map(|metadata| metadata.sequence)
                        .unwrap_or(0);

                    let (clock, flags) = if let Some(tp) = tp {
                        (
                            tp.into(),
                            wp_presentation_feedback::Kind::Vsync
                                | wp_presentation_feedback::Kind::HwClock
                                | wp_presentation_feedback::Kind::HwCompletion,
                        )
                    } else {
                        (self.clock.now(), wp_presentation_feedback::Kind::Vsync)
                    };

                    feedback.presented(
                        clock,
                        output
                            .current_mode()
                            .map(|mode| mode.refresh as u32)
                            .unwrap_or_default(),
                        seq as u64,
                        flags,
                    );
                }

                true
            }
            Err(err) => {
                warn!(self.log, "Error during rendering: {:?}", err);
                match err {
                    SwapBuffersError::AlreadySwapped => true,
                    SwapBuffersError::TemporaryFailure(err) => matches!(
                        err.downcast_ref::<DrmError>(),
                        Some(&DrmError::DeviceInactive)
                            | Some(&DrmError::Access {
                                source: drm::SystemError::PermissionDenied,
                                ..
                            })
                    ),
                    SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
                }
            }
        };

        if schedule_render {
            let output_refresh = match output.current_mode() {
                Some(mode) => mode.refresh,
                None => return,
            };
            // What are we trying to solve by introducing a delay here:
            //
            // Basically it is all about latency of client provided buffers.
            // A client driven by frame callbacks will wait for a frame callback
            // to repaint and submit a new buffer. As we send frame callbacks
            // as part of the repaint in the compositor the latency would always
            // be approx. 2 frames. By introducing a delay before we repaint in
            // the compositor we can reduce the latency to approx. 1 frame + the
            // remaining duration from the repaint to the next VBlank.
            //
            // With the delay it is also possible to further reduce latency if
            // the client is driven by presentation feedback. As the presentation
            // feedback is directly sent after a VBlank the client can submit a
            // new buffer during the repaint delay that can hit the very next
            // VBlank, thus reducing the potential latency to below one frame.
            //
            // Choosing a good delay is a topic on its own so we just implement
            // a simple strategy here. We just split the duration between two
            // VBlanks into two steps, one for the client repaint and one for the
            // compositor repaint. Theoretically the repaint in the compositor should
            // be faster so we give the client a bit more time to repaint. On a typical
            // modern system the repaint in the compositor should not take more than 2ms
            // so this should be safe for refresh rates up to at least 120 Hz. For 120 Hz
            // this results in approx. 3.33ms time for repainting in the compositor.
            // A too big delay could result in missing the next VBlank in the compositor.
            //
            // A more complete solution could work on a sliding window analyzing past repaints
            // and do some prediction for the next repaint.
            let repaint_delay =
                Duration::from_millis(((1_000_000f32 / output_refresh as f32) * 0.6f32) as u64);

            let timer = if self.udev_backend().primary_gpu != surface.render_node {
                // However, if we need to do a copy, that might not be enough.
                // (And without actual comparision to previous frames we cannot really know.)
                // So lets ignore that in those cases to avoid thrashing performance.
                trace!(
                    self.log,
                    "scheduling repaint timer immediately on {:?}",
                    crtc
                );
                Timer::immediate()
            } else {
                trace!(
                    self.log,
                    "scheduling repaint timer with delay {:?} on {:?}",
                    repaint_delay,
                    crtc
                );
                Timer::from_duration(repaint_delay)
            };

            self.handle
                .insert_source(timer, move |_, _, data| {
                    render::render(&mut data.state, dev_id, Some(crtc));
                    TimeoutAction::Drop
                })
                .expect("failed to schedule frame timer");
        }
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
        if let Some((_element, geo, bbox)) = element {
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
        _source: Option<smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource>,
        _icon: Option<WlSurface>,
        _seat: Seat<Self>,
    ) {
        info!(self.log, "ClientDndGrabHandler::started");
    }

    fn dropped(&mut self, _seat: Seat<Self>) {
        info!(self.log, "ClientDndGrabHandler::started");
    }
}
impl ServerDndGrabHandler for DWayState {
    fn action(
        &mut self,
        _action: smithay::reexports::wayland_server::protocol::wl_data_device_manager::DndAction,
    ) {
        info!(self.log, "ServerDndGrabHandler::action");
    }

    fn dropped(&mut self) {
        info!(self.log, "ServerDndGrabHandler::dropped");
    }

    fn cancelled(&mut self) {
        info!(self.log, "ServerDndGrabHandler::cancelled");
    }

    fn send(&mut self, _mime_type: String, _fd: OwnedFd) {
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
        _source: Option<smithay::reexports::wayland_server::protocol::wl_data_source::WlDataSource>,
    ) {
    }

    fn send_selection(&mut self, _mime_type: String, _fd: OwnedFd) {}
}
delegate_output!(DWayState);
delegate_primary_selection!(DWayState);
impl PrimarySelectionHandler for DWayState {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }

    fn new_selection(
        &mut self,
        _source: Option<smithay::reexports::wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_source_v1::ZwpPrimarySelectionSourceV1>,
    ) {
    }

    fn send_selection(&mut self, _mime_type: String, _fd: OwnedFd) {}
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
        _inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
    ) {
    }

    fn inhibitor_destroyed(
        &mut self,
        _inhibitor: smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitor,
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
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        _surface: WlSurface,
    ) {
        todo!()
    }

    fn destroy_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        _surface: WlSurface,
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

    fn request_mode(&mut self, _toplevel: ToplevelSurface, _mode: DecorationMode) {
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
        let rect = Rectangle::<i32, Logical>::from_loc_and_size((75, 75), (800, 600));
        let element = WindowElement::Wayland(Window::new(surface.clone()));
        place_new_window(&mut self.space, &element,rect.loc, true);
        with_surfaces_surface_tree(surface.wl_surface(), |s, states| {
            states.data_map.insert_if_missing(|| {
                let uuid = Uuid::new_v4();
                if s == surface.wl_surface() {
                    self.element_map.insert(uuid, element.clone());
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
            .and_then(DWaySurfaceData::get_logical_geometry_bbox)
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

    fn new_client(&mut self, _client: smithay::wayland::shell::xdg::ShellClient) {}

    fn client_pong(&mut self, _client: smithay::wayland::shell::xdg::ShellClient) {}

    fn move_request(
        &mut self,
        surface: ToplevelSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
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
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
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
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
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
        _surface: ToplevelSurface,
        _seat: smithay::reexports::wayland_server::protocol::wl_seat::WlSeat,
        _serial: smithay::utils::Serial,
        _location: Point<i32, Logical>,
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
        _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        _configure: smithay::wayland::shell::xdg::Configure,
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
        with_surfaces_surface_tree(surface.wl_surface(), |_surface, states| {
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
        with_surfaces_surface_tree(surface.wl_surface(), |_surface, states| {
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
        _surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        _output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
        _layer: smithay::wayland::shell::wlr_layer::Layer,
        _namespace: String,
    ) {
        todo!()
    }

    fn new_popup(
        &mut self,
        _parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        _popup: smithay::wayland::shell::xdg::PopupSurface,
    ) {
        todo!()
    }

    fn ack_configure(
        &mut self,
        _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        _configure: smithay::wayland::shell::wlr_layer::LayerSurfaceConfigure,
    ) {
    }

    fn layer_destroyed(&mut self, _surface: smithay::wayland::shell::wlr_layer::LayerSurface) {}
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
    fn xwm_state(&mut self, _xwm: smithay::xwayland::xwm::XwmId) -> &mut X11Wm {
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
        with_surfaces_surface_tree(&surface, |_surface, states| {
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
            with_surfaces_surface_tree(&surface, |_surface, states| {
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
        _x: Option<i32>,
        _y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<smithay::xwayland::xwm::Reorder>,
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
        _geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::reexports::x11rb::protocol::xproto::Window>,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
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
        _xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        _button: u32,
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
        _xwm: smithay::xwayland::xwm::XwmId,
        window: smithay::xwayland::X11Surface,
        _button: u32,
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

impl DmabufHandler for DWayState {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.udev_backend_mut().dmabuf_state.as_mut().unwrap().0
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        let primary_gpu = self.udev_backend().primary_gpu.clone();
        self.udev_backend_mut()
            .gpus
            .renderer::<Gles2Renderbuffer>(&primary_gpu, &primary_gpu)
            .and_then(|mut renderer| renderer.import_dmabuf(&dmabuf, None))
            .map(|_| ())
            .map_err(|_| ImportError::Failed)
    }
}
delegate_dmabuf!(DWayState);
