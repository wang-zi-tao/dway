#![feature(drain_filter)]
#![feature(result_flattening)]
#![feature(trivial_bounds)]
pub mod components;
pub mod egl;
pub mod keyboard;
pub mod layer;
pub mod log;
pub mod math;
pub mod pointer;
pub mod popup;
pub mod render;
pub mod surface;
// pub mod wayland;
pub mod cursor;
pub mod events;
pub mod fractional_scale;
pub mod input;
pub mod output;
pub mod placement;
pub mod presentation;
pub mod seat;
pub mod selection;
pub mod viewporter;
pub mod virtual_keyboard;
pub mod wayland_window;
pub mod x11_window;
pub mod xdg;

use std::{
    cell::RefCell,
    os::fd::AsRawFd,
    process::{self, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    ecs::query::WorldQuery,
    prelude::*,
    render::{pipelined_rendering::RenderExtractApp, RenderApp},
    sprite::SpriteSystem,
    ui::RenderUiSystem,
};
use components::{OutputWrapper, WlSurfaceWrapper};
// use bevy::prelude::*;
use failure::Fallible;
use log::logger;
use send_wrapper::SendWrapper;
use slog::Logger;
// use wayland::{
//     inputs::{receive_message, receive_messages},
//     render::render_desktop,
// };

// use self::wayland::{CalloopData, DWayState};
use crossbeam_channel::{Receiver, Sender};
use dway_protocol::window::WindowMessage;
use smithay::{
    backend::renderer::utils::RendererSurfaceState,
    delegate_fractional_scale, delegate_input_method_manager, delegate_presentation,
    delegate_text_input_manager,
    input::{keyboard::XkbConfig, pointer::CursorImageStatus, Seat, SeatState},
    output::{Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopHandle, Mode, PostAction},
        wayland_server::{
            backend::{smallvec::SmallVec, ClientData, ClientId, DisconnectReason},
            Display,
        },
    },
    utils::{Clock, Monotonic, Point},
    wayland::{
        compositor::{with_states, CompositorState},
        data_device::DataDeviceState,
        fractional_scale::FractionalScaleManagerState,
        input_method::{InputMethodManagerState, InputMethodSeat},
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        presentation::PresentationState,
        primary_selection::PrimarySelectionState,
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
        tablet_manager::TabletSeatTrait,
        text_input::TextInputManagerState,
        viewporter::ViewporterState,
        virtual_keyboard::VirtualKeyboardManagerState,
        xdg_activation::XdgActivationState,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};

use crate::{
    components::{PhysicalRect, SurfaceId, WindowIndex},
    cursor::Cursor,
    events::{
        CloseWindowRequest, CommitSurface, ConfigureX11WindowRequest, CreateTopLevelEvent,
        CreateWindow, CreateX11WindowEvent, DestroyWlSurface, KeyboardInputOnWindow,
        MouseButtonOnWindow, MouseMotionOnWindow, MouseWheelOnWindow, UnmapX11Window,
        UpdatePopupPosition, X11WindowSetSurfaceEvent,
    },
};

// pub fn main_loop(receiver: Receiver<WindowMessage>, sender: Sender<WindowMessage>) {
//     let log = logger();
//     // crate::wayland::backend::udev::run_udev(log,receiver, sender);
// }

#[derive(Resource)]
pub struct DWayBackend {
    pub log: Logger,
}

#[derive(Debug, Default)]
pub struct ClientState;
impl ClientData for ClientState {
    /// Notification that a client was initialized
    fn initialized(&self, client_id: ClientId) {
        info!("client {client_id:?} initialized");
    }
    /// Notification that a client is disconnected
    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        info!("disconnected client {client_id:?} , reason {reason:?}");
    }
}

pub struct DWay {
    // pub backend: Box<DWayBackend>,
    pub clock: Clock<Monotonic>,
    pub commands: Vec<Box<dyn FnOnce(&mut World) + Send + Sync>>,
    pub socket_name: String,
    pub display_number: Option<u32>,
    pub seat: Seat<Self>,

    pub xwayland: XWayland,
    pub seat_state: SeatState<Self>,
    pub xdg_shell: XdgShellState,
    pub xwm: Option<X11Wm>,
    pub compositor: CompositorState,
    pub shm_state: ShmState,
    pub data_device_state: DataDeviceState,
    pub wlr_layer_shell_state: WlrLayerShellState,
    pub output_manager_state: OutputManagerState,
    pub primary_selection_state: PrimarySelectionState,
    pub viewporter_state: ViewporterState,
    pub xdg_activation_state: XdgActivationState,
    pub xdg_decoration_state: XdgDecorationState,
    pub presentation_state: PresentationState,
    pub fractional_scale_manager_state: FractionalScaleManagerState,
    pub text_input_manger: TextInputManagerState,
    pub input_method_manager: InputMethodManagerState,
    // pub virtual_keyboard:VirtualKeyboardManagerState,
    // pub keyboard_shortcuts_inhibit_state : KeyboardShortcutsInhibitState,
}

impl DWay {
    pub fn new(
        display: &mut Display<Self>,
        handle: &LoopHandle<'static, DWayServerComponent>,
    ) -> Fallible<DWay> {
        let clock = Clock::new().expect("failed to initialize clock");
        let dh = display.handle();

        let source = ListeningSocketSource::new_auto().unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();

        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "winit");
        let cursor_status = Arc::new(Mutex::new(CursorImageStatus::Default));
        seat.add_pointer();
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to initialize the keyboard");

        let cursor_status2 = cursor_status.clone();
        seat.tablet_seat()
            .on_cursor_surface(move |_tool, new_status| {
                *cursor_status2.lock().unwrap() = new_status;
            });
        seat.add_input_method(XkbConfig::default(), 200, 25);

        handle.insert_source(source, |client_stream, _, data| {
            if let Err(err) = data
                .display
                .handle()
                .insert_client(client_stream, Arc::new(ClientState))
            {
                warn!("Error adding wayland client: {}", err);
            } else {
                info!("client connected");
            }
        })?;
        handle.insert_source(
            Generic::new(
                display.backend().poll_fd().as_raw_fd(),
                Interest::READ,
                Mode::Level,
            ),
            |_, _, data| {
                data.display.dispatch_clients(&mut data.dway).unwrap();
                Ok(PostAction::Continue)
            },
        )?;

        VirtualKeyboardManagerState::new::<Self, _>(&dh, |client| true);
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);
        let (xwayland, display_number) = x11_window::init(&dh, handle);
        Ok(DWay {
            // backend: todo!(),
            commands: Default::default(),
            socket_name,
            display_number,

            xwm: None,
            seat_state,
            seat,
            xwayland,
            data_device_state: DataDeviceState::new::<Self>(&dh),
            compositor: CompositorState::new::<Self>(&dh),
            xdg_shell: XdgShellState::new::<Self>(&dh),
            shm_state: ShmState::new::<Self>(&dh, vec![]),
            wlr_layer_shell_state: WlrLayerShellState::new::<Self>(&dh),
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&dh),
            primary_selection_state: PrimarySelectionState::new::<Self>(&dh),
            viewporter_state: ViewporterState::new::<Self>(&dh),
            xdg_activation_state: XdgActivationState::new::<Self>(&dh),
            xdg_decoration_state: XdgDecorationState::new::<Self>(&dh),
            presentation_state: PresentationState::new::<Self>(&dh, clock.id() as u32),
            fractional_scale_manager_state: FractionalScaleManagerState::new::<Self>(&dh),
            text_input_manger: TextInputManagerState::new::<Self>(&dh),
            input_method_manager: InputMethodManagerState::new::<Self>(&dh),

            clock,
        })
    }
    pub fn spawn(&self, mut command: process::Command) {
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
    pub fn update(&mut self) {}
    pub fn send_ecs_event<E: Send + Sync + 'static>(&mut self, e: E) {
        self.commands.push(Box::new(move |world| {
            world.send_event(e);
        }))
    }
}
#[derive(Resource)]
pub struct EventLoopResource(pub EventLoop<'static, DWayServerComponent>);

#[derive(Component)]
pub struct DWayServerComponent {
    pub dway: DWay,
    pub display: Display<DWay>,
}
#[derive(Resource)]
pub struct SeatWrapper {
    pub seat: Seat<DWay>,
}

pub fn new_backend(event_loop: NonSend<EventLoopResource>, mut commands: Commands) {
    let mut display = Display::new().unwrap();
    let handle = event_loop.0.handle();

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
    let _global = output.create_global::<DWay>(&display.handle());
    let mode = smithay::output::Mode {
        size: size.into(),
        refresh: 60_000,
    };
    output.change_current_state(Some(mode), None, None, Some((0, 0).into()));
    output.set_preferred(mode);
    commands.spawn(OutputWrapper(output));

    let dway = DWay::new(&mut display, &handle).unwrap();
    let mut command = process::Command::new("alacritty");
    command.args(&["-e", "htop", "-d", "2"]);
    let mut command = process::Command::new("weston-terminal");
    let mut command = process::Command::new("gnome-system-monitor");
    let mut command = process::Command::new("gnome-calculator");
    let mut command = process::Command::new("glxgears");
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    dway.spawn(command);
    commands.spawn(DWayServerComponent { dway, display });
}
pub fn spawn(dway_query: Query<&DWayServerComponent>) {
    let dway = dway_query.single();
    let mut command = process::Command::new("gnome-calculator");
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    dway.dway.spawn(command);
}
pub fn dispatch(
    mut event_loop: NonSendMut<EventLoopResource>,
    mut dway_query: Query<&mut DWayServerComponent>,
) {
    for mut dway in dway_query.iter_mut() {
        let result = event_loop
            .0
            .dispatch(Some(Duration::from_millis(16)), &mut dway);
        dway.display.flush_clients().unwrap();
        if let Err(e) = result {
            error!("{e}");
        }
    }
}
pub fn flush(world: &mut World) {
    let mut query: QueryState<&mut DWayServerComponent> = world.query();
    let mut commands = SmallVec::<[Box<dyn FnOnce(&mut World) + Send + Sync>; 8]>::new();
    for mut dway in query.iter_mut(world) {
        commands.extend(dway.dway.commands.drain(..));
    }
    for command in commands {
        command(world);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum DWayServerSystem {
    Dispatch,
    Flush,
    Create,
    CreateFlush,
    CreateComponent,
    CreateComponentFlush,
    PreUpdate,
    Update,
    PostUpdate,
    DestroyComponent,
    Destroy,
    DestroyFlush,
}

#[derive(Default)]
pub struct DWayServerPlugin {}
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use DWayServerSystem::*;

        app.configure_sets(
            (
                Dispatch,
                Flush,
                Create,
                CreateFlush,
                CreateComponent,
                CreateComponentFlush,
                PreUpdate,
                Update,
            )
                .in_base_set(CoreSet::PreUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (PostUpdate, DestroyComponent, Destroy, DestroyFlush)
                .in_base_set(CoreSet::PostUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.add_system(apply_system_buffers.in_set(CreateFlush));
        app.add_system(apply_system_buffers.in_set(CreateComponentFlush));
        app.add_system(apply_system_buffers.in_set(DestroyFlush));

        app.add_event::<events::CreateWindow>();
        app.add_event::<events::DestroyWindow>();
        app.add_event::<events::CreateTopLevelEvent>();
        app.add_event::<events::ConfigureWindowNotify>();
        app.add_event::<events::CreatePopup>();
        app.add_event::<events::DestroyPopup>();
        app.add_event::<events::DestroyWlSurface>();
        app.add_event::<events::CreateX11WindowEvent>();
        app.add_event::<events::MapX11Window>();
        app.add_event::<events::UnmapX11Window>();
        app.add_event::<events::MapOverrideX11Window>();
        app.add_event::<events::X11WindowSetSurfaceEvent>();
        app.add_event::<events::ConfigureX11WindowRequest>();
        app.add_event::<events::ConfigureX11WindowNotify>();
        app.add_event::<events::DestroyX11WindowEvent>();
        app.add_event::<events::WindowSetGeometryEvent>();
        app.add_event::<events::CommitSurface>();
        app.add_event::<events::ShowWindowMenu>();
        app.add_event::<events::MoveRequest>();
        app.add_event::<events::ResizeRequest>();
        app.add_event::<events::SetState>();
        app.add_event::<events::GrabPopup>();
        app.add_event::<events::UpdatePopupPosition>();
        app.add_event::<events::CloseWindowRequest>();
        app.add_event::<events::MouseMoveOnWindow>();
        app.add_event::<events::MouseMotionOnWindow>();
        app.add_event::<events::MouseButtonOnWindow>();
        app.add_event::<events::MouseWheelOnWindow>();
        app.add_event::<events::KeyboardInputOnWindow>();
        app.add_event::<events::NewDecoration>();
        app.add_event::<events::UnsetMode>();

        app.init_resource::<WindowIndex>();

        app.insert_non_send_resource(EventLoopResource(EventLoop::try_new().unwrap()));

        app.add_startup_system(new_backend);
        app.add_system(dispatch.in_set(Dispatch));
        app.add_system(flush.in_set(Flush));

        app.add_system(
            wayland_window::create_top_level
                .run_if(on_event::<CreateTopLevelEvent>())
                .in_set(Create),
        );
        app.add_system(
            wayland_window::destroy_wl_surface
                .run_if(on_event::<DestroyWlSurface>())
                .in_set(Destroy),
        );
        app.add_system(
            wayland_window::on_close_window_request
                .run_if(on_event::<CloseWindowRequest>())
                .in_set(PostUpdate),
        );
        app.add_system(wayland_window::on_state_changed.in_set(PostUpdate));
        app.add_system(wayland_window::on_rect_changed.in_set(PostUpdate));

        app.add_system(
            x11_window::create_x11_surface
                .run_if(on_event::<CreateX11WindowEvent>())
                .in_set(Create),
        );
        app.add_system(
            x11_window::map_x11_surface
                .run_if(on_event::<X11WindowSetSurfaceEvent>())
                .in_set(PreUpdate),
        );
        app.add_system(
            x11_window::unmap_x11_surface
                .run_if(on_event::<UnmapX11Window>())
                .in_set(PreUpdate),
        );
        app.add_system(x11_window::configure_notify.in_set(PreUpdate));
        app.add_system(
            x11_window::configure_request
                .run_if(on_event::<ConfigureX11WindowRequest>())
                .in_set(PreUpdate),
        );
        app.add_system(x11_window::on_rect_changed.in_set(PostUpdate));
        app.add_system(x11_window::on_state_changed.in_set(PostUpdate));
        app.add_system(
            x11_window::on_close_window_request
                .run_if(on_event::<CloseWindowRequest>())
                .in_set(PostUpdate),
        );

        app.add_system(
            popup::create_popup
                .run_if(on_event::<CreateWindow>())
                .in_set(Create),
        );
        app.add_system(
            popup::reposition_request
                .run_if(on_event::<UpdatePopupPosition>())
                .in_set(PreUpdate),
        );
        app.add_system(
            popup::on_commit
                .run_if(on_event::<CommitSurface>())
                .in_set(PreUpdate)
                .after(surface::do_commit),
        );

        app.add_system(
            surface::do_commit
                .run_if(on_event::<CommitSurface>())
                .in_set(PreUpdate),
        );
        app.add_system(
            surface::create_surface
                .run_if(on_event::<CreateWindow>())
                .in_set(CreateComponent),
        );
        app.add_system(surface::change_size.in_set(PostUpdate));

        app.add_system(
            placement::place_new_window
                .run_if(on_event::<CreateWindow>())
                .in_set(CreateComponent),
        );
        app.add_system(placement::update_logical_rect.in_set(Update));
        app.add_system(
            placement::update_global_physical_rect
                .after(placement::update_logical_rect)
                .in_set(Update),
        );

        app.add_system(input::on_mouse_move.in_set(PostUpdate));
        app.add_system(
            input::on_mouse_motion
                .run_if(on_event::<MouseMotionOnWindow>())
                .in_set(PostUpdate)
                .before(input::on_mouse_move),
        );
        app.add_system(
            input::on_mouse_button
                .run_if(on_event::<MouseButtonOnWindow>())
                .in_set(PostUpdate),
        );
        app.add_system(
            input::on_mouse_wheel
                .run_if(on_event::<MouseWheelOnWindow>())
                .in_set(PostUpdate),
        );
        app.add_system(
            input::on_keyboard
                .run_if(on_event::<KeyboardInputOnWindow>())
                .in_set(PostUpdate),
        );

        // app.add_system(print_window_list.before(Update));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_system(
                surface::import_surface
                    .in_schedule(ExtractSchedule)
                    .after(RenderUiSystem::ExtractNode)
                    .after(SpriteSystem::ExtractSprites), // .before(DWayRenderin_set::SendFrame),
            );
        }
    }
}
pub fn print_window_list(
    window_index: Res<WindowIndex>,
    mut query: Query<(&WlSurfaceWrapper)>,
    mut commands: Commands,
    ui_query: Query<(Entity, &Node, &UiImage)>,
) {
    // for (id, entity) in window_index.0.iter() {
    //     if let Ok((surface))=query.get(*entity){
    //         with_states(surface, |s| {
    //             dbg!(s);
    //             dbg!(s as *const _);
    //             dbg!(s.data_map.get::<RefCell<RendererSurfaceState>>());
    //         });
    //     }
    //     info!("surface {id:?} on {entity:?}");
    //     commands.entity(*entity).log_components();
    // }
    for e in ui_query.iter() {
        info!("ui image: {e:?}");
    }
}
delegate_presentation!(DWay);
delegate_text_input_manager!(DWay);
delegate_input_method_manager!(DWay);
