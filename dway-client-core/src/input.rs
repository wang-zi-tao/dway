use bevy::{
    core::FrameCount,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::*,
};
use bevy_relationship::{graph_query, ControlFlow};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    input::seat::SeatHasKeyboard,
    input::{
        grab::{Grab, GrabEvent, GrabEventKind},
        keyboard::XkbState,
        seat::{SeatHasPointer, WlSeat},
    },
    input::{keyboard::WlKeyboard, pointer::WlPointer},
    schedule::DWayServerSet,
    wl::surface::ClientHasSurface,
    wl::{region::WlRegion, surface::WlSurface},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, DWayWindow},
};

use crate::{desktop::CursorOnWindow, navigation::windowstack::WindowStack, DWayClientSystem};

use super::desktop::{CursorOnOutput, FocusedWindow};

#[derive(Default)]
pub struct DWayInputPlugin {
    pub debug: bool,
}
impl Plugin for DWayInputPlugin {
    fn build(&self, app: &mut App) {
        // app.add_system(print_pick_events.label(WindowLabel::Input));
        use DWayClientSystem::*;
        app.add_systems(
            PreUpdate,
            (
                mouse_move_on_window.run_if(on_event::<CursorMoved>()),
                cursor_move_on_window.run_if(on_event::<MouseMotion>()),
                mouse_button_on_window.run_if(on_event::<MouseButtonInput>()),
                mouse_wheel_on_window.run_if(on_event::<MouseWheel>()),
                keyboard_input_system.run_if(on_event::<KeyboardInput>()),
            )
                .in_set(DWayServerSet::Input),
        );
        if self.debug | true {
            app.add_systems(Startup, setup_debug_cursor);
            app.add_systems(PreUpdate, debug_follow_cursor.in_set(UpdateUI));
        }
    }
}
#[derive(Component)]
pub struct DebugCursor;

// #[tracing::instrument(skip_all)]
pub fn setup_debug_cursor(mut commands: Commands) {
    commands.spawn((
        DebugCursor,
        NodeBundle {
            background_color: Color::rgba_linear(0.5, 0.5, 0.5, 0.5).into(),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Px(16.0),
                height: Val::Px(16.0),
                ..default()
            },
            z_index: ZIndex::Global(512),
            ..default()
        },
    ));
}
// #[tracing::instrument(skip_all)]
pub fn debug_follow_cursor(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor: Query<&mut Style, With<DebugCursor>>,
) {
    for event in cursor_moved_events.iter() {
        let mut cursor = cursor.single_mut();
        cursor.left = Val::Px(event.position.x);
        cursor.top = Val::Px(event.position.y);
    }
}
graph_query!(KeyboardInputGraph=>[
    surface=(&'static WlSurface, &'static GlobalGeometry, Option<&'static XdgToplevel>, Option<&'static XdgPopup>),
    client=Entity,
    keyboard=&'static mut WlKeyboard,
]=>{
    path=surface<-[ClientHasSurface]-client-[SeatHasKeyboard]->keyboard
});
// #[tracing::instrument(skip_all)]
pub fn keyboard_input_system(
    mut graph: KeyboardInputGraph,
    mut keyboard_evens: EventReader<KeyboardInput>,
    output_focus: Res<FocusedWindow>,
    mut grab_events_writer: EventWriter<GrabEvent>,
    mut keystate: NonSendMut<XkbState>,
) {
    if keyboard_evens.is_empty() {
        return;
    }
    for event in keyboard_evens.iter() {
        keystate.key(event);
        if let Some(window) = output_focus.window_entity {
            graph.for_each_path_mut_from::<()>(
                window,
                |(surface, _rect, _toplevel, popup), _, keyboard| {
                    if popup.is_none() {
                        keyboard.key(surface, event, keystate.serialize());
                    }
                    ControlFlow::Continue
                },
            );
        }
        graph.node_keyboard.for_each_mut(|(entity, _)| {
            grab_events_writer.send(GrabEvent {
                seat_entity: entity,
                event_kind: GrabEventKind::Keyboard(*event, keystate.serialize()),
            });
        });
    }
}

pub fn mouse_move_on_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.iter() {
        focus.0 = Some((event.window, event.position.as_ivec2()));
    }
}

graph_query!(PointInputGraph=>[
    surface=< (Entity, &'static WlSurface, &'static GlobalGeometry, Option<&'static XdgToplevel>, Option<&'static XdgPopup>),With<DWayWindow>>,
    client=(Entity,&'static mut WlSeat,&'static mut Grab),
    pointer=(Entity, &'static mut WlPointer,&'static mut Geometry ),
]=>{
    path=surface<-[ClientHasSurface]-client-[SeatHasPointer]->pointer
});

fn cursor_move_on_window(
    mut graph: PointInputGraph,
    region_query: Query<&WlRegion>,
    cursor: Res<CursorOnOutput>,
    window_stack: Res<WindowStack>,
    mut cursor_on_window: ResMut<CursorOnWindow>,
    mut grab_events_writer: EventWriter<GrabEvent>,
    mut events: EventReader<MouseMotion>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        return;
    };
    for MouseMotion { delta: _ } in events.iter() {
        for window in window_stack.list.iter() {
            if graph
                .for_each_path_mut_from::<()>(
                    *window,
                    |(surface_entity, surface, rect, _toplevel, popup),
                     (_seat_entity, ref mut seat, ref mut _geab),
                     (_pointer_entity, pointer, pointer_rect)| {
                        pointer_rect.set_pos(*pos);
                        if popup.is_none() {
                            let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                            if !rect.include_point(*pos)
                                || surface
                                    .commited
                                    .input_region
                                    .and_then(|region| region_query.get(region).ok())
                                    .map(|region| !region.is_inside(relative))
                                    .unwrap_or(false)
                            {
                                if seat.can_focus_on(surface) {
                                    // pointer.leave();
                                }
                                return ControlFlow::Continue;
                            }
                            pointer.move_cursor(seat, surface, relative.as_vec2());
                            cursor_on_window.0 = Some((*surface_entity, relative));
                            return ControlFlow::Return(());
                        }
                        ControlFlow::Continue
                    },
                )
                .is_some()
            {
                break;
            };
        }
        graph
            .node_client
            .for_each_mut(|(_, (entity, mut seat, ..))| {
                seat.pointer_position = Some(*pos);
                grab_events_writer.send(GrabEvent {
                    seat_entity: entity,
                    event_kind: dway_server::input::grab::GrabEventKind::PointerMove(pos.as_vec2()),
                });
            });
    }
}

fn mouse_button_on_window(
    mut graph: PointInputGraph,
    mut events: EventReader<MouseButtonInput>,
    cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    cursor_on_window: Res<CursorOnWindow>,
    mut grab_events_writer: EventWriter<GrabEvent>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        if let Some((window, _)) = cursor_on_window.0.as_ref() {
            graph.for_each_path_mut_from::<()>(
                *window,
                |(surface_entity, surface, rect, _toplevel, popup),
                 (_seat_entity, seat, ref mut grab),
                 (_pointer_entity, pointer, _)| {
                    if popup.is_none() {
                        if !rect.include_point(*pos) {
                            return ControlFlow::Continue;
                        };
                        let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                        output_focus.window_entity = Some(*surface_entity);

                        if matches!(&**grab, Grab::OnPopup { .. }) {
                            pointer.button(seat, e, surface, relative.as_dvec2());
                        } else if e.state == ButtonState::Pressed {
                            **grab = Grab::ButtonDown {
                                surface: *surface_entity,
                            };
                            seat.enable();
                            seat.grab(surface);
                        }
                    }
                    ControlFlow::Continue
                },
            );
        }
        graph.node_client.for_each(|(_, (entity, ..))| {
            grab_events_writer.send(GrabEvent {
                seat_entity: entity,
                event_kind: GrabEventKind::PointerButton(*e, pos.as_dvec2()),
            });
        });
    }
}
fn mouse_wheel_on_window(
    mut graph: PointInputGraph,
    mut events: EventReader<MouseWheel>,
    cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    cursor_on_window: Res<CursorOnWindow>,
    mut grab_events_writer: EventWriter<GrabEvent>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        if let Some((window, _)) = cursor_on_window.0.as_ref() {
            graph.for_each_path_mut_from::<()>(
                *window,
                |(surface_entity, surface, rect, toplevel, _popup),
                 (_seat_entity, ref mut seat, ref mut _grab),
                 (_pointer_entity, pointer, ..)| {
                    if toplevel.is_some() {
                        if !rect.include_point(*pos) {
                            return ControlFlow::Continue;
                        };
                        let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                        let acc = |x: f64| x * 20.0;
                        pointer.asix(
                            seat,
                            DVec2::new(-acc(e.x as f64), -acc(e.y as f64)),
                            surface,
                            relative.as_dvec2(),
                        );
                        output_focus.window_entity = Some(*surface_entity);
                    }
                    ControlFlow::Continue
                },
            );
        }
        graph.node_client.for_each(|(_, (entity, ..))| {
            grab_events_writer.send(GrabEvent {
                seat_entity: entity,
                event_kind: GrabEventKind::PointerAxis(*e, pos.as_dvec2()),
            });
        });
    }
}
