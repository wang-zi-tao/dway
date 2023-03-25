use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{hash_map::Entry, HashMap},
    ffi::OsString,
    rc::Rc,
    sync::{atomic::Ordering, Mutex},
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessage;
use slog::{crit, error, info, warn, Logger};
use smithay::{
    backend::{
        drm::{DrmDevice, DrmDeviceFd, DrmNode, GbmBufferedSurface, NodeType},
        egl::{EGLContext, EGLDevice, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage::{DamageTrackedRenderer, DamageTrackedRendererError},
            element::{
                default_primary_scanout_output_compare, texture::TextureBuffer, AsRenderElements,
                RenderElementStates,
            },
            gles2::{Gles2Renderbuffer, Gles2Renderer},
            multigpu::{egl::EglGlesBackend, GpuManager, MultiRenderer, MultiTexture},
            Bind, Frame, ImportDma, ImportEgl, Renderer,
        },
        session::{self, libseat::LibSeatSession, Session},
        udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent},
        SwapBuffersError,
    },
    desktop::{
        space::SurfaceTree,
        utils::{
            surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
            update_surface_primary_scanout_output, OutputPresentationFeedback,
        },
        Space,
    },
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{Dispatcher, EventLoop, LoopHandle, RegistrationToken},
        drm::{
            self,
            control::{connector, crtc, encoder, Device},
        },
        gbm,
        input::Libinput,
        wayland_server::{backend::GlobalId, protocol::wl_surface, Display, DisplayHandle},
    },
    utils::{Clock, IsAlive, Logical, Monotonic, Point, Rectangle, Scale, Transform},
    wayland::{
        compositor,
        dmabuf::{DmabufGlobal, DmabufState},
        fractional_scale::with_fractional_scale,
        input_method::InputMethodHandle,
    },
};

use crate::wayland::{
    backend::Backend,
    cursor::{Cursor, PointerElement, CLEAR_COLOR},
    inputs::process_input_event,
    render::{render, render_output},
    shell::WindowElement,
    CalloopData, DWayState,
};
#[derive(Debug, PartialEq)]
pub struct UdevOutputId {
    pub device_id: DrmNode,
    pub crtc: crtc::Handle,
}

pub type UdevRenderer<'a> = MultiRenderer<
    'a,
    'a,
    'a,
    EglGlesBackend<Gles2Renderer>,
    Gles2Renderbuffer,
>;
pub type RenderSurface =
    GbmBufferedSurface<gbm::Device<DrmDeviceFd>, Option<OutputPresentationFeedback>>;
pub struct SurfaceData {
    pub dh: DisplayHandle,
    pub device_id: DrmNode,
    pub render_node: DrmNode,
    pub surface: RenderSurface,
    pub global: Option<GlobalId>,
    pub damage_tracked_renderer: DamageTrackedRenderer,
}

impl Drop for SurfaceData {
    fn drop(&mut self) {
        if let Some(global) = self.global.take() {
            self.dh.remove_global::<DWayState>(global);
        }
    }
}

pub struct BackendData {
    pub surfaces: Rc<RefCell<HashMap<crtc::Handle, Rc<RefCell<SurfaceData>>>>>,
    pub gbm: gbm::Device<DrmDeviceFd>,
    pub registration_token: RegistrationToken,
    pub event_dispatcher: Dispatcher<'static, DrmDevice, CalloopData>,
}

pub struct UDevBackend {
    pub session: LibSeatSession,
    pub dh: DisplayHandle,
    pub dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    pub primary_gpu: DrmNode,
    pub gpus: GpuManager<EglGlesBackend<Gles2Renderer>>,
    pub backends: HashMap<DrmNode, BackendData>,
    pub pointer_images: Vec<(xcursor::parser::Image, TextureBuffer<MultiTexture>)>,
    pub pointer_element: PointerElement<MultiTexture>,
    pub pointer_image: Cursor,
    pub logger: slog::Logger,
}
impl std::fmt::Debug for UDevBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UDevBackend")
            .field("session", &self.session)
            .field("dh", &self.dh)
            .field("dmabuf_state", &self.dmabuf_state)
            .field("primary_gpu", &self.primary_gpu)
            .field("gpus", &self.gpus)
            // .field("backends", &self.backends)
            .field("pointer_images", &self.pointer_images)
            // .field("pointer_element", &self.pointer_element)
            // .field("fps_texture", &self.fps_texture)
            // .field("pointer_image", &self.pointer_image)
            .field("logger", &self.logger)
            .finish()
    }
}
pub fn run_udev(log: Logger, receiver: Receiver<WindowMessage>, sender: Sender<WindowMessage>) {
    // let mut event_loop = EventLoop::try_new().unwrap();
    // let mut display = Display::new().unwrap();
    //
    // /*
    //  * Initialize session
    //  */
    // let (session, notifier) = match LibSeatSession::new(log.clone()) {
    //     Ok(ret) => ret,
    //     Err(err) => {
    //         crit!(log, "Could not initialize a session: {}", err);
    //         return;
    //     }
    // };
    //
    // /*
    //  * Initialize the compositor
    //  */
    // let primary_gpu = if let Ok(var) = std::env::var("ANVIL_DRM_DEVICE") {
    //     DrmNode::from_path(var).expect("Invalid drm device path")
    // } else {
    //     primary_gpu(&session.seat())
    //         .unwrap()
    //         .and_then(|x| {
    //             DrmNode::from_path(x)
    //                 .ok()?
    //                 .node_with_type(NodeType::Render)?
    //                 .ok()
    //         })
    //         .unwrap_or_else(|| {
    //             all_gpus(&session.seat())
    //                 .unwrap()
    //                 .into_iter()
    //                 .find_map(|x| DrmNode::from_path(x).ok())
    //                 .expect("No GPU!")
    //         })
    // };
    // info!(log, "Using {} as primary gpu.", primary_gpu);
    //
    // let mut gpus = GpuManager::new(EglGlesBackend::default(), log.clone()).unwrap();
    // let mut renderer = gpus
    //     .renderer::<Gles2Renderbuffer>(&primary_gpu, &primary_gpu)
    //     .unwrap();
    //
    // // init dmabuf support with format list from our primary gpu
    // // TODO: This does not necessarily depend on egl, but mesa makes no use of it without wl_drm right now
    // let dmabuf_state = {
    //     info!(
    //         log,
    //         "Trying to initialize EGL Hardware Acceleration via {:?}", primary_gpu
    //     );
    //
    //     if renderer.bind_wl_display(&display.handle()).is_ok() {
    //         info!(log, "EGL hardware-acceleration enabled");
    //         let dmabuf_formats = renderer.dmabuf_formats().cloned().collect::<Vec<_>>();
    //         let mut state = DmabufState::new();
    //         let global =
    //             state.create_global::<DWayState, _>(&display.handle(), dmabuf_formats, );
    //         Some((state, global))
    //     } else {
    //         None
    //     }
    // };
    //
    // let data = UDevBackend {
    //     dh: display.handle(),
    //     dmabuf_state,
    //     session,
    //     primary_gpu,
    //     gpus,
    //     backends: HashMap::new(),
    //     pointer_image: Cursor::load(&log),
    //     pointer_images: Vec::new(),
    //     pointer_element: PointerElement::default(),
    //     logger: log.clone(),
    // };
    // let mut state = DWayState::init(
    //     &mut display,
    //     event_loop.handle(),
    //     Backend::UDev(data),
    //     log.clone(),
    //     receiver,
    //     sender,
    // );
    //
    // /*
    //  * Initialize the udev backend
    //  */
    // let udev_backend = match UdevBackend::new(&state.backend.seat_name(), ) {
    //     Ok(ret) => ret,
    //     Err(err) => {
    //         crit!(log, "Failed to initialize udev backend"; "error" => err);
    //         return;
    //     }
    // };
    // dbg!(udev_backend.device_list().collect::<Vec<_>>());
    //
    // /*
    //  * Initialize a fake output (we render one screen to every device in this example)
    //  */
    //
    // /*
    //  * Initialize libinput backend
    //  */
    // let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
    //     if let Backend::UDev(u) = &mut state.backend {
    //         u.session.clone().into()
    //     } else {
    //         panic!()
    //     },
    // );
    // libinput_context
    //     .udev_assign_seat(&state.backend.seat_name())
    //     .unwrap();
    // let libinput_backend = LibinputInputBackend::new(libinput_context.clone(), );
    //
    // /*
    //  * Bind all our objects that get driven by the event loop
    //  */
    // event_loop
    //     .handle()
    //     .insert_source(libinput_backend, move |event, _, data| {
    //         let dh = data.state.udev_backend_mut().dh.clone();
    //         process_input_event(&mut data.state, &dh, event)
    //     })
    //     .unwrap();
    // let handle = event_loop.handle();
    // let log2 = log.clone();
    // event_loop
    //     .handle()
    //     .insert_source(notifier, move |event, &mut (), data| match event {
    //         session::Event::PauseSession => {
    //             slog::info!(log2, "session::Event::PauseSession");
    //             libinput_context.suspend();
    //             for backend in data.state.udev_backend_mut().backends.values() {
    //                 backend.event_dispatcher.as_source_ref().pause();
    //             }
    //         }
    //         session::Event::ActivateSession => {
    //             slog::info!(log2, "session::Event::ActivateSession");
    //             if let Err(err) = libinput_context.resume() {
    //                 slog::error!(log2, "Failed to resume libinput context: {:?}", err);
    //             }
    //             for (node, backend) in data
    //                 .state
    //                 .udev_backend_mut()
    //                 .backends
    //                 .iter()
    //                 .map(|(handle, backend)| (*handle, backend))
    //             {
    //                 backend.event_dispatcher.as_source_ref().activate();
    //                 let surfaces = backend.surfaces.borrow();
    //                 for surface in surfaces.values() {
    //                     if let Err(err) = surface.borrow().surface.surface().reset_state() {
    //                         slog::warn!(log2, "Failed to reset drm surface state: {}", err);
    //                     }
    //                 }
    //                 handle.insert_idle(move |data| {
    //                     render(&mut data.state, node, None)
    //                 });
    //             }
    //         }
    //     })
    //     .unwrap();
    // for (dev, path) in udev_backend.device_list() {
    //     slog::info!(state.log, "add device {:?} {:?}", dev, path);
    //     state.device_added(&mut display, dev, path.into())
    // }
    //
    // let log2 = log.clone();
    // event_loop
    //     .handle()
    //     .insert_source(udev_backend, move |event, _, data| match event {
    //         UdevEvent::Added { device_id, path } => {
    //             slog::info!(log2, "add device {:?} {:?}", device_id, path);
    //             data.state.device_added(&mut data.display, device_id, path)
    //         }
    //         UdevEvent::Changed { device_id } => {
    //             slog::info!(log2, "changed device {:?}", device_id);
    //             data.state.device_changed(&mut data.display, device_id)
    //         }
    //         UdevEvent::Removed { device_id } => {
    //             slog::info!(log2, "remove device {:?}", device_id);
    //             data.state.device_removed(device_id)
    //         }
    //     })
    //     .unwrap();
    //
    // /*
    //  * Start XWayland if supported
    //  */
    // if let Err(e) = state.xwayland.start(
    //     state.handle.clone(),
    //     None,
    //     std::iter::empty::<(OsString, OsString)>(),
    //     |_| {},
    // ) {
    //     error!(log, "Failed to start XWayland: {}", e);
    // }
    //
    // /*
    //  * And run our loop
    //  */
    //
    // while state.running.load(Ordering::SeqCst) {
    //     let mut calloop_data = CalloopData { state, display };
    //     let result = event_loop.dispatch(Some(Duration::from_millis(16)), &mut calloop_data);
    //     CalloopData { state, display } = calloop_data;
    //
    //     let log2 = state.log.clone();
    //     for (node, backend) in state
    //         .udev_backend()
    //         .backends
    //         .iter()
    //         .map(|(handle, backend)| (*handle, backend))
    //     {
    //         backend.event_dispatcher.as_source_ref().activate();
    //         let surfaces = backend.surfaces.borrow();
    //         for surface in surfaces.values() {
    //             if let Err(err) = surface.borrow().surface.surface().reset_state() {
    //                 slog::warn!(log2, "Failed to reset drm surface state: {}", err);
    //             }
    //         }
    //         state.handle.insert_idle(move |data| {
    //             render(&mut data.state, node, None)
    //         });
    //     }
    //     if result.is_err() {
    //         state.running.store(false, Ordering::SeqCst);
    //     } else {
    //         state.space.refresh();
    //         state.popups.cleanup();
    //         display.flush_clients().unwrap();
    //     }
    // }
}

pub fn post_repaint(
    output: &Output,
    render_element_states: &RenderElementStates,
    space: &Space<WindowElement>,
    time: impl Into<Duration>,
) {
    let time = time.into();
    let throttle = Some(Duration::from_secs(1));

    space.elements().for_each(|window| {
        window.with_surfaces(|surface, states| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states,
                render_element_states,
                default_primary_scanout_output_compare,
            );

            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fraction_scale| {
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        if space.outputs_for_element(window).contains(output) {
            window.send_frame(output, time, throttle, surface_primary_scanout_output);
        }
    });
    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.with_surfaces(|surface, states| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states,
                render_element_states,
                default_primary_scanout_output_compare,
            );

            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fraction_scale| {
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        layer_surface.send_frame(output, time, throttle, surface_primary_scanout_output);
    }
}

pub fn take_presentation_feedback(
    output: &Output,
    space: &Space<WindowElement>,
    render_element_states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

    space.elements().for_each(|window| {
        if space.outputs_for_element(window).contains(output) {
            window.take_presentation_feedback(
                &mut output_presentation_feedback,
                surface_primary_scanout_output,
                |surface, _| {
                    surface_presentation_feedback_flags_from_states(surface, render_element_states)
                },
            );
        }
    });
    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.take_presentation_feedback(
            &mut output_presentation_feedback,
            surface_primary_scanout_output,
            |surface, _| {
                surface_presentation_feedback_flags_from_states(surface, render_element_states)
            },
        );
    }

    output_presentation_feedback
}
pub fn chedule_initial_render(
    gpus: &mut GpuManager<EglGlesBackend<Gles2Renderer>>,
    surface: Rc<RefCell<SurfaceData>>,
    evt_handle: &LoopHandle<'static, CalloopData>,
    logger: ::slog::Logger,
) {
    // let node = surface.borrow().render_node;
    // let result = {
    //     let mut renderer = gpus.renderer::<Gles2Renderbuffer>(&node, &node).unwrap();
    //     let mut surface = surface.borrow_mut();
    //     initial_render(&mut surface.surface, &mut renderer)
    // };
    // if let Err(err) = result {
    //     match err {
    //         SwapBuffersError::AlreadySwapped => {}
    //         SwapBuffersError::TemporaryFailure(err) => {
    //             // TODO dont reschedule after 3(?) retries
    //             warn!(logger, "Failed to submit page_flip: {}", err);
    //             let handle = evt_handle.clone();
    //             evt_handle.insert_idle(move |data| match &mut data.state.backend {
    //                 crate::wayland::backend::Backend::UDev(u) => {
    //                     schedule_initial_render(&mut u.gpus, surface, &handle, logger)
    //                 }
    //                 _ => panic!(),
    //             });
    //         }
    //         SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
    //     }
    // }
}

fn initial_render(
    surface: &mut RenderSurface,
    renderer: &mut UdevRenderer<'_>,
) -> Result<(), SwapBuffersError> {
    let (dmabuf, _) = surface.next_buffer()?;
    renderer.bind(dmabuf)?;
    // Does not matter if we render an empty frame
    let mut frame = renderer
        .render((1, 1).into(), Transform::Normal)
        .map_err(Into::<SwapBuffersError>::into)?;
    frame.clear(CLEAR_COLOR, &[Rectangle::from_loc_and_size((0, 0), (1, 1))])?;
    frame.finish().map_err(Into::<SwapBuffersError>::into)?;
    surface.queue_buffer(None)?;
    surface.reset_buffers();
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn scan_connectors(
    device_id: DrmNode,
    device: &DrmDevice,
    gbm: &gbm::Device<DrmDeviceFd>,
    display: &mut Display<DWayState>,
    space: &mut Space<WindowElement>,
    #[cfg(feature = "debug")] fps_texture: &MultiTexture,
    logger: &::slog::Logger,
) -> HashMap<crtc::Handle, Rc<RefCell<SurfaceData>>> {
    // Get a set of all modesetting resource handles (excluding planes):
    let res_handles = device.resource_handles().unwrap();

    // Find all connected output ports.
    let connector_infos: Vec<connector::Info> = res_handles
        .connectors()
        .iter()
        .map(|conn| device.get_connector(*conn, true).unwrap())
        .filter(|conn| conn.state() == connector::State::Connected)
        .inspect(|conn| info!(logger, "Connected: {:?}", conn.interface()))
        .collect();

    let mut backends = HashMap::new();

    let (render_node, formats) = {
        let display = EGLDisplay::new(gbm.clone(), ).unwrap();
        let node = match EGLDevice::device_for_display(&display)
            .ok()
            .and_then(|x| x.try_get_render_node().ok().flatten())
        {
            Some(node) => node,
            None => return HashMap::new(),
        };
        let context = EGLContext::new(&display, ).unwrap();
        (node, context.dmabuf_render_formats().clone())
    };

    // very naive way of finding good crtc/encoder/connector combinations. This problem is np-complete
    for connector_info in connector_infos {
        let encoder_infos = connector_info
            .encoders()
            .iter()
            .flat_map(|encoder_handle| device.get_encoder(*encoder_handle))
            .collect::<Vec<encoder::Info>>();

        let crtcs = encoder_infos
            .iter()
            .flat_map(|encoder_info| res_handles.filter_crtcs(encoder_info.possible_crtcs()));

        for crtc in crtcs {
            // Skip CRTCs used by previous connectors.
            let entry = match backends.entry(crtc) {
                Entry::Vacant(entry) => entry,
                Entry::Occupied(_) => continue,
            };

            info!(
                logger,
                "Trying to setup connector {:?}-{} with crtc {:?}",
                connector_info.interface(),
                connector_info.interface_id(),
                crtc,
            );

            let mode = connector_info.modes()[0];
            let surface = match device.create_surface(crtc, mode, &[connector_info.handle()]) {
                Ok(surface) => surface,
                Err(err) => {
                    warn!(logger, "Failed to create drm surface: {}", err);
                    continue;
                }
            };

            let gbm_surface = match GbmBufferedSurface::new(
                surface,
                gbm.clone(),
                formats.clone(),
            ) {
                Ok(renderer) => renderer,
                Err(err) => {
                    warn!(logger, "Failed to create rendering surface: {}", err);
                    continue;
                }
            };

            let size = mode.size();
            let mode = Mode {
                size: (size.0 as i32, size.1 as i32).into(),
                refresh: mode.vrefresh() as i32 * 1000,
            };

            let interface_short_name = match connector_info.interface() {
                drm::control::connector::Interface::DVII => Cow::Borrowed("DVI-I"),
                drm::control::connector::Interface::DVID => Cow::Borrowed("DVI-D"),
                drm::control::connector::Interface::DVIA => Cow::Borrowed("DVI-A"),
                drm::control::connector::Interface::SVideo => Cow::Borrowed("S-VIDEO"),
                drm::control::connector::Interface::DisplayPort => Cow::Borrowed("DP"),
                drm::control::connector::Interface::HDMIA => Cow::Borrowed("HDMI-A"),
                drm::control::connector::Interface::HDMIB => Cow::Borrowed("HDMI-B"),
                drm::control::connector::Interface::EmbeddedDisplayPort => Cow::Borrowed("eDP"),
                other => Cow::Owned(format!("{:?}", other)),
            };

            let output_name = format!("{}-{}", interface_short_name, connector_info.interface_id());

            let (phys_w, phys_h) = connector_info.size().unwrap_or((0, 0));
            let output = Output::new(
                output_name,
                PhysicalProperties {
                    size: (phys_w as i32, phys_h as i32).into(),
                    subpixel: Subpixel::Unknown,
                    make: "Smithay".into(),
                    model: "Generic DRM".into(),
                },
            );
            let global = output.create_global::<DWayState>(&display.handle());
            let position = (
                space
                    .outputs()
                    .fold(0, |acc, o| acc + space.output_geometry(o).unwrap().size.w),
                0,
            )
                .into();
            output.change_current_state(Some(mode), None, None, Some(position));
            output.set_preferred(mode);
            space.map_output(&output, position);

            output
                .user_data()
                .insert_if_missing(|| UdevOutputId { crtc, device_id });

            let damage_tracked_renderer = DamageTrackedRenderer::from_output(&output);

            entry.insert(Rc::new(RefCell::new(SurfaceData {
                dh: display.handle(),
                device_id,
                render_node,
                surface: gbm_surface,
                global: Some(global),
                damage_tracked_renderer,
            })));

            break;
        }
    }

    backends
}

pub fn schedule_initial_render(
    gpus: &mut GpuManager<EglGlesBackend<Gles2Renderer>>,
    surface: Rc<RefCell<SurfaceData>>,
    evt_handle: &LoopHandle<'static, CalloopData>,
    logger: ::slog::Logger,
) {
    // let node = surface.borrow().render_node;
    // let result = {
    //     let mut renderer = gpus.renderer::<Gles2Renderbuffer>(&node, &node).unwrap();
    //     let mut surface = surface.borrow_mut();
    //     initial_render(&mut surface.surface, &mut renderer)
    // };
    // if let Err(err) = result {
    //     match err {
    //         SwapBuffersError::AlreadySwapped => {}
    //         SwapBuffersError::TemporaryFailure(err) => {
    //             // TODO dont reschedule after 3(?) retries
    //             warn!(logger, "Failed to submit page_flip: {}", err);
    //             let handle = evt_handle.clone();
    //             evt_handle.insert_idle(move |data| {
    //                 schedule_initial_render(
    //                     &mut data.state.udev_backend_mut().gpus,
    //                     surface,
    //                     &handle,
    //                     logger,
    //                 )
    //             });
    //         }
    //         SwapBuffersError::ContextLost(err) => panic!("Rendering loop lost: {}", err),
    //     }
    // }
}
