use std::time::SystemTime;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    winit::WinitWindows, utils::tracing,
};
// use bevy_mod_picking::{PickingEvent, PickingRaycastSet};
// use bevy_mod_raycast::Intersection;
use dway_server::{
    components::{Id, PhysicalRect, SurfaceId, WindowMark, WindowScale, GlobalPhysicalRect, WlSurfaceWrapper},
    events::{
        KeyboardInputOnWindow, MouseButtonOnWindow, MouseMotionOnWindow, MouseMoveOnWindow,
        MouseWheelOnWindow,
    },
    math::{ivec2_to_point, vec2_to_point},
};
use log::info;

use dway_protocol::window::{WindowMessage, WindowMessageKind};
use smithay::utils::Physical;

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

#[tracing::instrument(skip_all)]
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
#[tracing::instrument(skip_all)]
pub fn debug_follow_cursor(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: NonSend<WinitWindows>,
    mut cursor: Query<&mut Style, With<DebugCursor>>,
) {
    for event in cursor_moved_events.iter() {
        let Some( window )=windows.get_window(event.window)else{
            error!("failed to get window {:?}",event.window);
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

#[tracing::instrument(skip_all)]
pub fn print_mouse_events_system(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
) {
    for event in mouse_button_input_events.iter() {
        info!("{:?}", event);
    }

    for event in mouse_motion_events.iter() {
        info!("{:?}", event);
    }

    for event in cursor_moved_events.iter() {
        info!("{:?}", event);
    }

    for event in mouse_wheel_events.iter() {
        info!("{:?}", event);
    }
}
#[tracing::instrument(skip_all)]
pub fn keyboard_input_system(
    mut keyboard_evens: EventReader<KeyboardInput>,
    output_focus: Res<FocusedWindow>,
    surface_id_query: Query<&SurfaceId>,
    mut sender: EventWriter<KeyboardInputOnWindow>,
) {
    if keyboard_evens.is_empty() {
        return;
    }
    let Some(focus_window)=&output_focus.0 else{
        warn!("no focus window");
        return;
    };
    let Ok( id )=surface_id_query.get(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    for event in keyboard_evens.iter() {
        sender.send(KeyboardInputOnWindow(id.clone(), event.clone()));
    }
}
#[tracing::instrument(skip_all)]
pub fn mouse_move_on_winit_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: NonSend<WinitWindows>,
    mut focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.iter() {
        let Some( window )=windows.get_window(event.window)else{
            error!("failed to get window {:?}",event.window);
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
fn cursor_move_on_window(
    mut interaction_query: Query<(&Interaction, &Backend), With<Button>>,
    mut surfaces_query: Query<(&SurfaceId, &GlobalPhysicalRect, Option<&WindowScale>), (With<WindowMark>,With<WlSurfaceWrapper>)>,
    mut cursor: Res<CursorOnOutput>,
    mut events_writer: EventWriter<MouseMoveOnWindow>,
    mut motion_events_writer: EventWriter<MouseMotionOnWindow>,
    mut events: EventReader<MouseMotion>,
) {
    for MouseMotion { delta } in events.iter() {
        for (interaction, backend) in &mut interaction_query {
            match *interaction {
                Interaction::None => {}
                _ => {
                    let Ok((id, rect, window_scale)) = surfaces_query.get(backend.0) else{
                        // warn!("failed to get backend");
                        continue;
                    };
                    let Some((output, pos)) = &cursor.0  else {
                        warn!("no cursor position data");
                        continue;
                    };
                    let relative = ivec2_to_point(*pos) - rect.0.loc;
                    let scale = window_scale.cloned().unwrap_or_default().0;
                    let logical = relative.to_f64().to_logical(scale).to_i32_round();
                    events_writer.send(MouseMoveOnWindow(id.clone(), logical));
                    motion_events_writer.send(MouseMotionOnWindow(
                        id.clone(),
                        vec2_to_point::<Physical>(*delta)
                            .to_f64()
                            .to_logical(scale)
                            .to_i32_round(),
                    ));
                }
            }
        }
    }
}
fn mouse_button_on_window(
    mut interaction_query: Query<(&Interaction, &Backend), With<Button>>,
    mut surfaces_query: Query<(&SurfaceId, &GlobalPhysicalRect, Option<&WindowScale>), (With<WindowMark>,With<WlSurfaceWrapper>)>,
    mut events: EventReader<MouseButtonInput>,
    mut cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    mut events_writer: EventWriter<MouseButtonOnWindow>,
) {
    for e in events.iter() {
        for (interaction, backend) in &mut interaction_query {
            match *interaction {
                Interaction::None => {}
                _ => {
                    let Ok((id, rect, window_scale)) = surfaces_query.get(backend.0) else{
                        warn!("failed to get backend");
                        continue;
                    };
                    let Some((output, pos)) = &cursor.0  else {
                        warn!("no cursor position data");
                        continue;
                    };
                    let relative = ivec2_to_point(*pos) - rect.0.loc;
                    let scale = window_scale.cloned().unwrap_or_default().0;
                    let logical = relative.to_f64().to_logical(scale).to_i32_round();
                    events_writer.send(MouseButtonOnWindow(id.clone(), logical, e.clone()));
                    output_focus.0 = Some(backend.get());
                }
            }
        }
    }
}
fn mouse_wheel_on_window(
    mut interaction_query: Query<(&Interaction, &Backend), With<Button>>,
    mut surfaces_query: Query<(&SurfaceId, &GlobalPhysicalRect, Option<&WindowScale>), (With<WindowMark>,With<WlSurfaceWrapper>)>,
    mut events: EventReader<MouseWheel>,
    mut cursor: Res<CursorOnOutput>,
    mut output_focus: ResMut<FocusedWindow>,
    mut events_writer: EventWriter<MouseWheelOnWindow>,
) {
    for e in events.iter() {
        for (interaction, backend) in &mut interaction_query {
            match *interaction {
                Interaction::None => {}
                _ => {
                    let Ok((id, rect, window_scale)) = surfaces_query.get(backend.0) else{
                        warn!("failed to get backend");
                        continue;
                    };
                    let Some((output, pos)) = &cursor.0  else {
                        warn!("no cursor position data");
                        continue;
                    };
                    let relative = ivec2_to_point(*pos) - rect.0.loc;
                    let scale = window_scale.cloned().unwrap_or_default().0;
                    let logical = relative.to_f64().to_logical(scale).to_i32_round();
                    events_writer.send(MouseWheelOnWindow(id.clone(), logical, e.clone()));
                    output_focus.0 = Some(backend.get());
                }
            }
        }
    }
}
