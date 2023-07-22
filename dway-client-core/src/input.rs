use std::time::SystemTime;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    utils::tracing,
    winit::WinitWindows,
};
use bevy_relationship::{graph_query, Connectable, EntityHasChildren};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    input::seat::SeatHasKeyboard,
    input::{
        grab::{PointerAxisGrabEvent, PointerButtonGrabEvent, PointerGrab, PointerMoveGrabEvent},
        seat::SeatHasPointer,
    },
    input::{keyboard::WlKeyboard, pointer::WlPointer},
    util::rect::IRect,
    wl::surface::ClientHasSurface,
    wl::{region::WlRegion, surface::WlSurface},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, XdgSurface},
};
// use bevy_mod_picking::{PickingEvent, PickingRaycastSet};
// use bevy_mod_raycast::Intersection;
use log::info;

use dway_protocol::window::{WindowMessage, WindowMessageKind};

use crate::{window::Backend, DWayClientSystem};

use super::desktop::{CursorOnOutput, FocusedWindow};

#[derive(Default)]
pub struct DWayInputPlugin {
    pub debug: bool,
}
impl Plugin for DWayInputPlugin {
    fn build(&self, app: &mut App) {
        // app.add_system(print_pick_events.label(WindowLabel::Input));
        use DWayClientSystem::*;
        app.add_system(
            mouse_move_on_winit_window
                .run_if(on_event::<CursorMoved>())
                .in_set(Create),
        );
        app.add_system(
            cursor_move_on_window
                .run_if(on_event::<MouseMotion>())
                .in_set(Input),
        );
        app.add_system(
            mouse_button_on_window
                .run_if(on_event::<MouseButtonInput>())
                .in_set(Input),
        );
        app.add_system(
            mouse_wheel_on_window
                .run_if(on_event::<MouseWheel>())
                .in_set(Input),
        );
        app.add_system(
            keyboard_input_system
                .run_if(on_event::<KeyboardInput>())
                .in_set(Input),
        );
        if self.debug {
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
    windows: NonSend<WinitWindows>,
    mut cursor: Query<&mut Style, With<DebugCursor>>,
) {
    for event in cursor_moved_events.iter() {
        let Some(window) = windows.get_window(event.window) else {
            error!("failed to get window {:?}", event.window);
            continue;
        };
        let pos: Vec2 = (
            event.position.x,
            window.inner_size().height as f32 - event.position.y,
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
    surface=(&'static WlSurface, &'static GlobalGeometry),
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
) {
    if keyboard_evens.is_empty() {
        return;
    }
    let Some(focus_window) = &output_focus.0 else {
        warn!("no focus window");
        return;
    };
    for event in keyboard_evens.iter() {
        graph.for_each_path_mut(|(surface, rect), _, keyboard| {
            keyboard.key(surface, event);
        });
    }
}
// #[tracing::instrument(skip_all)]
pub fn mouse_move_on_winit_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: NonSend<WinitWindows>,
    mut focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.iter() {
        let Some(window) = windows.get_window(event.window) else {
            error!("failed to get window {:?}", event.window);
            continue;
        };
        let pos: IVec2 = (
            event.position.x as i32,
            window.inner_size().height as i32 - event.position.y as i32,
        )
            .into();
        focus.0 = Some((event.window, pos));
    }
}
graph_query!(PointInputGraph=>[
    surface=< (Entity, &'static WlSurface, &'static GlobalGeometry, Option<&'static XdgToplevel>, Option<&'static XdgPopup>),With<WlSurface> >,
    client=Entity,
    pointer=(Entity, &'static mut WlPointer,&'static mut Geometry ),
]=>{
    path=surface<-[ClientHasSurface]-client-[SeatHasPointer]->pointer
});

fn cursor_move_on_window(
    mut graph: PointInputGraph,
    mut region_query: Query<&WlRegion>,
    mut cursor: Res<CursorOnOutput>,
    mut grab_events_writer: EventWriter<PointerMoveGrabEvent>,
    mut events: EventReader<MouseMotion>,
) {
    let Some((output, pos)) = &cursor.0 else {
        return;
    };
    for MouseMotion { delta } in events.iter() {
        graph.for_each_path_mut(
            |(surface_entity, surface, rect, toplevel, popup),
             _,
             pointer: &mut (Entity, Mut<'_, WlPointer>, Mut<'_, Geometry>)| {
                grab_events_writer.send(PointerMoveGrabEvent(pointer.0, pos.as_vec2()));

                if popup.is_none() {
                    if !rect.include_point(*pos)
                        || surface
                            .commited
                            .input_region
                            .and_then(|region| region_query.get(region).ok())
                            .map(|region| {
                                !region.is_inside(
                                    *pos - rect.geometry.pos() - surface.image_rect().pos(),
                                )
                            })
                            .unwrap_or(false)
                    {
                        pointer.1.leave();
                        return;
                    }
                    let relative = *pos - rect.geometry.pos() - surface.image_rect().pos();
                    pointer.1.move_cursor(surface, relative.as_vec2());
                    pointer.2.set_pos(*pos);
                    // info!("mouse move: {:?}", relative);
                }
            },
        );
    }
}

fn mouse_button_on_window(
    mut graph: PointInputGraph,
    mut events: EventReader<MouseButtonInput>,
    mut cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    mut grab_events_writer: EventWriter<PointerButtonGrabEvent>,
) {
    let Some((output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        graph.for_each_path(
            |(surface_entity, surface, rect, toplevel, popup), _, (pointer_entity, pointer, _)| {
                grab_events_writer.send(PointerButtonGrabEvent(
                    *pointer_entity,
                    *e,
                    pos.as_dvec2(),
                ));

                if true {
                    if !rect.include_point(*pos) {
                        return;
                    };
                    let relative = *pos - rect.geometry.pos();
                    output_focus.0 = Some(*surface_entity);
                    pointer.button(e);
                    debug!("mouse button: {:?}", e);
                }
            },
        );
    }
}
fn mouse_wheel_on_window(
    mut graph: PointInputGraph,
    mut events: EventReader<MouseWheel>,
    mut cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    mut grab_events_writer: EventWriter<PointerAxisGrabEvent>,
) {
    let Some((output, pos)) = &cursor.0 else {
        warn!("no cursor position data");
        return;
    };
    for e in events.iter() {
        graph.for_each_path(
            |(surface_entity, surface, rect, toplevel, popup), _, (pointer_entity, pointer, _)| {
                grab_events_writer.send(PointerAxisGrabEvent(*pointer_entity, *e, pos.as_dvec2()));

                if let Some(toplevel) = toplevel {
                    if !rect.include_point(*pos) {
                        return;
                    };
                    let relative = *pos - rect.geometry.pos();
                    if e.x != 0.0 {
                        pointer.horizontal_asix(e.x as f64);
                    }
                    if e.y != 0.0 {
                        pointer.horizontal_asix(e.y as f64);
                    }
                    output_focus.0 = Some(*surface_entity);
                }
                debug!("mouse wheel: {:?}", e);
            },
        );
    }
}
