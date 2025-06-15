use bevy::{
    ecs::{
        event::EventCursor,
        system::{RunSystemOnce, SystemId},
    },
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::*,
};
use bevy_relationship::{graph_query, ControlFlow};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    input::{
        grab::{ResizeEdges, StartGrab, WlSurfacePointerState},
        keyboard::{WlKeyboard, XkbState},
        pointer::WlPointer,
        seat::{SeatHasKeyboard, SeatHasPointer, WlSeat},
    },
    macros::WindowAction,
    schedule::DWayServerSet,
    util::rect::IRect,
    wl::surface::{ClientHasSurface, WlSurface},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, DWayWindow},
};

use super::desktop::{CursorOnScreen, FocusedWindow};
use crate::{desktop::CursorOnWindow, DWayClientSystem};

#[derive(Default)]
pub struct DWayInputPlugin {
    pub debug: bool,
}
impl Plugin for DWayInputPlugin {
    fn build(&self, app: &mut App) {
        // app.add_system(print_pick_events.label(WindowLabel::Input));

        app.add_event::<SurfaceInputEvent>();
        app.init_resource::<GrabManagerSystems>();
        app.init_resource::<GrabManager>();
        app.add_systems(
            PostUpdate,
            (
                on_start_grab_event.run_if(on_event::<StartGrab>),
                on_input_event
                    .run_if(on_event::<SurfaceInputEvent>)
                    .before(mouse_move_on_window),
                mouse_move_on_window.run_if(on_event::<CursorMoved>),
            )
                .chain()
                .in_set(DWayClientSystem::Input),
        );
        app.register_type::<SurfaceUiNode>();
    }
}

#[derive(Resource)]
pub struct GrabManagerSystems {
    pub move_window: SystemId<In<GrabRequest>, GrabResponse>,
    pub resize_window: SystemId<In<GrabRequest>, GrabResponse>,
}

impl FromWorld for GrabManagerSystems {
    fn from_world(world: &mut World) -> Self {
        let move_window = world.register_system(move_grab);
        let resize_window = world.register_system(resize_grab);

        Self {
            move_window,
            resize_window,
        }
    }
}

pub fn on_start_grab_event(
    mut events: EventReader<StartGrab>,
    mut commands: Commands,
    systems: Res<GrabManagerSystems>,
    mut grab_manager: ResMut<GrabManager>,
) {
    for event in events.read() {
        match event {
            StartGrab::Move {
                surface,
                seat,
                serial,
                mouse_pos,
                geometry,
            } => {
                let entity = commands
                    .spawn(GrabMoveWindow {
                        mouse_offset: mouse_pos.as_vec2(),
                        begin_position: geometry.pos(),
                    })
                    .id();
                grab_manager.grab = Some((entity, systems.move_window));
            }
            StartGrab::Resizing {
                surface,
                seat,
                edges,
                serial,
                geometry,
            } => {
                let entity = commands
                    .spawn(GrabResizeWindow {
                        edges: *edges,
                        begin_rect: geometry.geometry,
                        begin_geometry: geometry.clone(),
                    })
                    .id();
                grab_manager.grab = Some((entity, systems.resize_window));
            }
            StartGrab::Drag {
                surface,
                seat,
                data_device,
                icon,
            } => todo!(),
        }
    }
}

structstruck::strike! {
    #[derive(Event,Debug,Clone)]
    pub struct SurfaceInputEvent {
        pub surface_entity: Option<Entity>,
        pub mouse_position: Vec2,
        pub surface_rect: Rect,
        pub kind: #[derive(Debug,Clone)] enum GrabRequestKind {
            Move(Vec2),
            Button(MouseButtonInput),
            Asix(MouseWheel),
            Enter(),
            Leave(),
            KeyboardInput(KeyboardInput),
            WindowAction(WindowAction),
        }
    }
}

structstruck::strike! {
    #[derive(Debug,Clone)]
    pub struct GrabRequest {
        pub grab_entity: Entity,
        pub event: SurfaceInputEvent,
    }
}

#[derive(Default, Debug)]
pub struct GrabResponse {
    block_event: bool,
    finish: bool,
}

#[derive(Resource, Default)]
pub struct GrabManager {
    pub grab: Option<(Entity, SystemId<In<GrabRequest>, GrabResponse>)>,
}

impl GrabManager {
    pub fn process(world: &mut World, event: SurfaceInputEvent) -> GrabResponse {
        let this = world.resource::<Self>();
        if let Some((grab_entity, grab_system)) = this.grab {
            let response = world
                .run_system_with_input(grab_system, GrabRequest { grab_entity, event })
                .unwrap();
            if response.finish {
                world.despawn(grab_entity);
                let mut this = world.resource_mut::<Self>();
                this.grab = None;
            }
            response
        } else {
            GrabResponse::default()
        }
    }
}

graph_query!(InputGraph=>[
    surface=< (&'static WlSurface,&'static mut WlSurfacePointerState, Option<&'static XdgPopup>),With<DWayWindow>>,
    client=&'static mut WlSeat,
    pointer=&'static mut WlPointer,
    keyboard=&'static mut WlKeyboard,
]=>{
    pointer=surface<-[ClientHasSurface]-client-[SeatHasPointer]->pointer,
    keyboard=surface<-[ClientHasSurface]-client-[SeatHasKeyboard]->keyboard,
});

pub fn do_input(
    In(event): In<SurfaceInputEvent>,
    mut graph: InputGraph,
    mut cursor_on_window: ResMut<CursorOnWindow>,
    mut output_focus: ResMut<FocusedWindow>,
    mut keystate: NonSendMut<XkbState>,
) {
    let Some(surface_entity) = event.surface_entity else {
        return;
    };

    if let GrabRequestKind::KeyboardInput(keyboard_input) = &event.kind {
        graph.for_each_keyboard_mut_from::<()>(
            surface_entity,
            |(surface, _seat, popup), _, keyboard| {
                if popup.is_none() {
                    keyboard.key(surface, keyboard_input, keystate.serialize());
                }
                ControlFlow::Continue
            },
        );
        return;
    }

    graph.for_each_pointer_mut_from::<()>(
        surface_entity,
        |(surface, window_pointer, popup), ref mut seat, pointer| {
            let relative_pos = event.mouse_position;

            match &event.kind {
                GrabRequestKind::Move(_cursor_moved) => {
                    pointer.move_cursor(seat, surface, relative_pos);
                    window_pointer.mouse_pos = relative_pos.as_ivec2();
                    cursor_on_window.0 = Some((surface_entity, relative_pos.as_ivec2()));
                }
                GrabRequestKind::Button(mouse_button_input) => {
                    output_focus.window_entity = Some(surface_entity);
                    pointer.button(seat, mouse_button_input, surface, relative_pos);
                    if !event.surface_rect.contains(event.mouse_position) {
                        if let Some(popup) = popup {
                            popup.raw.popup_done();
                        }
                    }
                }
                GrabRequestKind::Asix(mouse_wheel) => {
                    let acc = |x: f64| x * 20.0;
                    pointer.asix(
                        seat,
                        DVec2::new(-acc(mouse_wheel.x as f64), -acc(mouse_wheel.y as f64)),
                        surface,
                        relative_pos,
                    );
                    output_focus.window_entity = Some(surface_entity);
                }
                GrabRequestKind::Enter() => {
                    pointer.enter(seat, surface, relative_pos);
                }
                GrabRequestKind::Leave() => {
                    pointer.leave();
                }
                GrabRequestKind::WindowAction(_window_action) => {}
                GrabRequestKind::KeyboardInput(_) => {
                    unreachable!();
                }
            };
            ControlFlow::default()
        },
    );
}

#[derive(Component, Debug)]
pub struct GrabMoveWindow {
    pub mouse_offset: Vec2,
    pub begin_position: IVec2,
}

pub fn move_grab(
    In(request): In<GrabRequest>,
    mut surface_query: Query<(&Geometry,)>,
    mut window_action: EventWriter<WindowAction>,
    grab_query: Query<&GrabMoveWindow>,
) -> GrabResponse {
    let event = &request.event;
    let Some(surface_entity) = event.surface_entity else {
        return default();
    };
    let Ok(GrabMoveWindow {
        mouse_offset,
        begin_position,
    }) = grab_query.get(request.grab_entity)
    else {
        return default();
    };

    let mut response = GrabResponse {
        block_event: true,
        ..Default::default()
    };

    let _ = surface_query
        .get_mut(surface_entity)
        .map(|(window_geometry,)| match &event.kind {
            GrabRequestKind::Move(cursor_position) => {
                let pos = (cursor_position - mouse_offset + window_geometry.pos().as_vec2()).as_ivec2();
                window_action.send(WindowAction::SetRect(
                    surface_entity,
                    IRect {
                        min: pos,
                        max: pos + window_geometry.size(),
                    },
                ));
            }
            GrabRequestKind::Button(mouse_button_input) => {
                if mouse_button_input.state == ButtonState::Released {
                    response.finish = true;
                }
            }
            _ => {}
        });
    response
}

#[derive(Component, Debug)]
pub struct GrabResizeWindow {
    pub edges: ResizeEdges,
    pub begin_rect: IRect,
    pub begin_geometry: Geometry,
}

pub fn resize_grab(
    In(request): In<GrabRequest>,
    mut surface_query: Query<(
        &WlSurface,
        &mut WlSurfacePointerState,
        &Geometry,
        Option<&XdgPopup>,
    )>,
    mut window_action: EventWriter<WindowAction>,
    grab_query: Query<&GrabResizeWindow>,
) -> GrabResponse {
    let event = &request.event;
    let Some(surface_entity) = event.surface_entity else {
        return default();
    };
    let Ok(GrabResizeWindow {
        edges,
        begin_rect,
        begin_geometry,
    }) = grab_query.get(request.grab_entity)
    else {
        return default();
    };

    let mut response = GrabResponse {
        block_event: true,
        ..Default::default()
    };

    let _ = surface_query.get_mut(surface_entity).map(
        |(surface, mut window_pointer, window_geometry, popup)| match &event.kind {
            GrabRequestKind::Move(cursor_position) => {
                let mut geo = begin_geometry.geometry;
                let pos = cursor_position + event.surface_rect.min - window_geometry.pos().as_vec2();
                if edges.contains(ResizeEdges::LEFT) {
                    geo.min.x = pos.x as i32;
                }
                if edges.contains(ResizeEdges::TOP) {
                    geo.min.y = pos.y as i32;
                }
                if edges.contains(ResizeEdges::RIGHT) {
                    geo.max.x = pos.x as i32;
                }
                if edges.contains(ResizeEdges::BUTTOM) {
                    geo.max.y = pos.y as i32;
                }
                window_action.send(WindowAction::SetRect(surface_entity, geo));
            }
            GrabRequestKind::Button(mouse_button_input) => {
                if mouse_button_input.state == ButtonState::Released {
                    response.finish = true;
                }
            }
            _ => {}
        },
    );
    response
}

pub fn on_input_event(world: &mut World, mut events_cursor: Local<EventCursor<SurfaceInputEvent>>) {
    let events_resource = world.resource::<Events<_>>();
    let events: Vec<_> = events_cursor.read(events_resource).cloned().collect();
    for event in events {
        let response = GrabManager::process(world, event.clone());
        if response.block_event {
            continue;
        }

        let _ = world.run_system_cached_with(do_input, event);
    }
}

pub fn mouse_move_on_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut focus: ResMut<CursorOnScreen>,
) {
    for event in cursor_moved_events.read() {
        focus.0 = Some((event.window, event.position.as_ivec2()));
    }
}

#[derive(Component, Debug, Reflect)]
pub struct SurfaceUiNode {
    pub surface_entity: Entity,
    pub widget: Entity,
    pub grab: bool,
}

impl SurfaceUiNode {
    pub fn new(surface_entity: Entity, widget: Entity) -> Self {
        Self {
            surface_entity,
            widget,
            grab: false,
        }
    }

    pub fn with_grab(mut self, grab: bool) -> Self {
        self.grab = grab;
        self
    }
}
