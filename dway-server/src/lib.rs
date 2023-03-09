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
    ecs::{query::WorldQuery, schedule::ReportExecutionOrderAmbiguities},
    prelude::*,
    render::{RenderApp, RenderStage},
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
    input::{Seat, SeatState},
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
        input_method::InputMethodManagerState,
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
        text_input::TextInputManagerState,
        viewporter::ViewporterState,
        virtual_keyboard::VirtualKeyboardManagerState,
        xdg_activation::XdgActivationState,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};

use crate::{components::WindowIndex, cursor::Cursor};

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
        let logger = logger();
        let dh = display.handle();

        let source = ListeningSocketSource::new_auto().unwrap();
        let socket_name = source.socket_name().to_string_lossy().into_owned();

        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "winit");

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
        let (xwayland, channel) = XWayland::new(&dh);
        x11_window::init(channel, &dh, handle);

        VirtualKeyboardManagerState::new::<Self, _>(&dh, |_client| true);
        // let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);
        Ok(DWay {
            // backend: todo!(),
            commands: Default::default(),
            socket_name,
            display_number: None,

            xwm: None,
            seat_state,
            seat,
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
pub fn new_backend(event_loop: NonSend<EventLoopResource>, mut commands: Commands) {
    let mut display = Display::new().unwrap();
    let handle = event_loop.0.handle();
    let dway = DWay::new(&mut display, &handle).unwrap();
    let mut command = process::Command::new("gnome-system-monitor");
    let mut command = process::Command::new("gnome-calculator");
    let mut command = process::Command::new("alacritty");
    command.args(&["-e", "htop", "-d", "2"]);
    command.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    dway.spawn(command);
    commands.spawn(DWayServerComponent { dway, display });

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
    commands.spawn(OutputWrapper(output));
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum DWayInitLabel {
    Server,
    Process,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum DWayRenderLabel {
    Import,
    SendFrame,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum DWayServerLabel {
    Dispatch,
    Flush,
    Create,
    Update,
    Destroy,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum DWayServerStage {
    Receive,
    Send,
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

#[derive(Default)]
pub struct DWayServerPlugin {}
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use DWayServerLabel::*;

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
        app.add_event::<events::MouseButtonOnWindow>();
        app.add_event::<events::NewDecoration>();
        app.add_event::<events::UnsetMode>();

        app.init_resource::<WindowIndex>();

        app.insert_non_send_resource(EventLoopResource(EventLoop::try_new().unwrap()));

        let mut receive_stage = SystemStage::single_threaded();
        app.add_stage_before(
            CoreStage::PreUpdate,
            DWayServerStage::Receive,
            receive_stage,
        );
        let mut send_stage = SystemStage::parallel();
        app.add_stage_after(CoreStage::PostUpdate, DWayServerStage::Send, send_stage);

        app.add_startup_system(new_backend.label(DWayInitLabel::Server));
        // app.add_startup_system(
        //     spawn
        //         .label(DWayInitLabel::Process)
        //         .after(DWayInitLabel::Server),
        // );
        app.add_system_to_stage(DWayServerStage::Receive, dispatch.label(Dispatch));
        app.add_system_to_stage(
            DWayServerStage::Receive,
            flush.label(Flush).before(Create).after(Dispatch),
        );

        app.add_system_to_stage(
            DWayServerStage::Receive,
            wayland_window::create_top_level
                .label(Create)
                .before(Update),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            wayland_window::destroy_wl_surface
                .label(Destroy)
                .after(Update),
        );
        app.add_system(wayland_window::on_close_window_request.label(Update));
        app.add_system(wayland_window::on_state_changed.label(Update));
        app.add_system(wayland_window::on_rect_changed.label(Update));

        app.add_system_to_stage(
            DWayServerStage::Receive,
            x11_window::create_x11_surface.label(Create),
        );
        app.add_system(x11_window::map_x11_surface.label(Update));
        app.add_system(x11_window::unmap_x11_surface.label(Update));
        app.add_system(x11_window::configure_notify.label(Update));
        app.add_system(x11_window::configure_request.label(Update));
        app.add_system_to_stage(
            DWayServerStage::Send,
            x11_window::on_rect_changed.label(Update),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            x11_window::on_state_changed.label(Update),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            x11_window::on_close_window_request.label(Update),
        );

        app.add_system_to_stage(
            DWayServerStage::Receive,
            popup::create_popup.label(Create).before(Update),
        );
        app.add_system(popup::reposition_request.label(Update));
        app.add_system(popup::on_commit.label(Update));

        app.add_system(surface::on_commit.label(Update));
        app.add_system_to_stage(
            DWayServerStage::Receive,
            surface::create_surface.before(Update).after(Create),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            surface::change_size.before(Destroy).after(Update),
        );
        // app.add_system_to_stage(DWayServerStage::Send, surface::clean_damage.label(Destroy));
        // app.add_system_to_stage(DWayServerStage::Receive, surface::send_frame);

        app.add_system_to_stage(
            DWayServerStage::Receive,
            placement::place_new_window.before(Update).after(Create),
        );
        app.add_system_to_stage(
            DWayServerStage::Receive,
            placement::update_global_physical_rect.label(Update),
        );
        app.add_system_to_stage(
            DWayServerStage::Send,
            placement::update_physical_rect.label(Update),
        );

        // app.add_system(print_window_list.before(Update));

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(ReportExecutionOrderAmbiguities);
            render_app.add_system_to_stage(
                RenderStage::Extract,
                surface::import_surface
                    // .pipe(surface::send_frame)
                    // .label(DWayRenderLabel::Import)
                    .after(RenderUiSystem::ExtractNode)
                    .after(SpriteSystem::ExtractSprites), // .before(DWayRenderLabel::SendFrame),
            );
            // render_app.add_system_to_stage(
            //     RenderStage::Extract,
            //     surface::debug_texture
            //         .label(DWayRenderLabel::SendFrame)
            //         .after(DWayRenderLabel::Import),
            // );
            // render_app.add_system_to_stage(
            //     RenderStage::Extract,
            //     surface::send_frame.after(DWayRenderLabel::Import),
            // );
        }
    }
}
delegate_presentation!(DWay);
delegate_text_input_manager!(DWay);
delegate_input_method_manager!(DWay);
