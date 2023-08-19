use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::*,
};
use bevy_relationship::graph_query;
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    input::seat::SeatHasKeyboard,
    input::{
        grab::{Grab, GrabEvent, GrabEventKind},
        seat::{SeatHasPointer, WlSeat},
    },
    input::{keyboard::WlKeyboard, pointer::WlPointer},
    schedule::DWayServerSet,
    wl::surface::ClientHasSurface,
    wl::{region::WlRegion, surface::WlSurface},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, DWayWindow},
};

use crate::DWayClientSystem;

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
            (
                mouse_move_on_winit_window.run_if(on_event::<CursorMoved>()),
                cursor_move_on_window.run_if(on_event::<MouseMotion>()),
                mouse_button_on_window.run_if(on_event::<MouseButtonInput>()),
                mouse_wheel_on_window.run_if(on_event::<MouseWheel>()),
                keyboard_input_system.run_if(on_event::<KeyboardInput>()),
            )
                .in_set(DWayServerSet::Input),
        );
        if self.debug | true {
            app.add_startup_system(setup_debug_cursor);
            app.add_system(debug_follow_cursor.in_set(UpdateUI));
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
                position: UiRect {
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                size: Size::new(Val::Px(16.0), Val::Px(16.0)),
                ..default()
            },
            z_index: ZIndex::Global(1024),
            ..default()
        },
    ));
}
// #[tracing::instrument(skip_all)]
pub fn debug_follow_cursor(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window>,
    mut cursor: Query<&mut Style, With<DebugCursor>>,
) {
    for event in cursor_moved_events.iter() {
        let Ok(window) = windows.get(event.window) else {
            error!("failed to get window {:?}", event.window);
            continue;
        };
        let pos: Vec2 = (
            event.position.x,
            window.physical_height() as f32 - event.position.y,
        )
            .into();
        let mut cursor = cursor.single_mut();
        cursor.position = UiRect {
            left: Val::Px(pos.x),
            top: Val::Px(pos.y),
            ..default()
        };
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
) {
    if keyboard_evens.is_empty() {
        return;
    }
    let Some(_focus_window) = &output_focus.0 else {
        trace!("no focus window");
        return;
    };
    for event in keyboard_evens.iter() {
        graph.for_each_path_mut(|(surface, _rect, _toplevel, popup), _, keyboard| {
            if popup.is_none() {
                keyboard.key(surface, event);
            }
        });
        graph.node_keyboard.for_each_mut(|(entity, _)| {
            grab_events_writer.send(GrabEvent {
                seat_entity: entity,
                event_kind: GrabEventKind::Keyboard(*event),
            })
        });
    }
}
// #[tracing::instrument(skip_all)]
pub fn mouse_move_on_winit_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Query<&Window>,
    mut focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.iter() {
        let Ok(window) = windows.get(event.window) else {
            error!("failed to get window {:?}", event.window);
            continue;
        };
        let pos = IVec2::new(
            event.position.x as i32,
            window.physical_height() as i32 - event.position.y as i32,
        );
        focus.0 = Some((event.window, pos));
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
    mut grab_events_writer: EventWriter<GrabEvent>,
    mut events: EventReader<MouseMotion>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        return;
    };
    for MouseMotion { delta: _ } in events.iter() {
        graph.for_each_path_mut(
            |(_surface_entity, surface, rect, _toplevel, popup),
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
                        return;
                    }
                    pointer.move_cursor(seat, surface, relative.as_vec2());
                }
            },
        );
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
    mut grab_events_writer: EventWriter<GrabEvent>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        graph.for_each_path_mut(
            |(surface_entity, surface, rect, _toplevel, popup),
             (_seat_entity, seat, ref mut grab),
             (_pointer_entity, pointer, _)| {
                if popup.is_none() {
                    if !rect.include_point(*pos) {
                        return;
                    };
                    let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                    output_focus.0 = Some(*surface_entity);

                    if matches!(&**grab, Grab::OnPopup { .. }) {
                        pointer.button(seat, e, surface, relative.as_dvec2());
                    } else if e.state == ButtonState::Pressed {
                        **grab = Grab::ButtonDown {
                            surface: *surface_entity,
                        };
                        seat.enable();
                        seat.grab(surface);
                    } else {
                        // pointer.button(seat, e, surface, relative.as_dvec2());
                    }
                }
            },
        );
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
    mut grab_events_writer: EventWriter<GrabEvent>,
) {
    let Some((_output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        graph.for_each_path_mut(
            |(surface_entity, surface, rect, toplevel, _popup),
             (_seat_entity, ref mut seat, ref mut _grab),
             (_pointer_entity, pointer, ..)| {
                if toplevel.is_some() {
                    if !rect.include_point(*pos) {
                        return;
                    };
                    let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                    let acc = |x: f64| x * 20.0;
                    pointer.asix(
                        seat,
                        DVec2::new(-acc(e.x as f64), -acc(e.y as f64)),
                        surface,
                        relative.as_dvec2(),
                    );
                    output_focus.0 = Some(*surface_entity);
                }
            },
        );
        graph.node_client.for_each(|(_, (entity, ..))| {
            grab_events_writer.send(GrabEvent {
                seat_entity: entity,
                event_kind: GrabEventKind::PointerAxis(*e, pos.as_dvec2()),
            });
        });
    }
}
