use std::time::SystemTime;

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseMotion},
        ButtonState,
    },
    prelude::*,
    winit::WinitWindows,
};
use dway_protocol::window::{WindowMessage, WindowMessageKind};
use dway_server::{
    components::{
        GlobalPhysicalRect, PhysicalRect, SurfaceId, WindowIndex, WindowMark, WindowScale,
    },
    events::MoveRequest,
    math::{ivec2_to_point, point_to_vec2, vec2_to_point},
};

use crate::{
    desktop::{CursorOnOutput, FocusedWindow},
    protocol::WindowMessageSender,
    window::Backend,
    DWayClientState, DWayClientSystem,
};
#[derive(Resource)]
pub struct MovingState {
    relatice: Vec2,
    backend: Entity,
}
#[derive(Default)]
pub struct DWayMovingPlugin {}
impl Plugin for DWayMovingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(start_moving.run_if(on_event::<MoveRequest>()));
        app.add_system(
            move_window
                .run_if(on_event::<CursorMoved>().and_then(resource_exists::<MovingState>()))
                .in_set(OnUpdate(DWayClientState::Moving)),
        );
        app.add_system(
            stop_moving
                .run_if(on_event::<MouseButtonInput>())
                .after(move_window)
                .in_set(OnUpdate(DWayClientState::Moving)),
        );
    }
}
pub fn start_moving(
    output_focus: Res<CursorOnOutput>,
    mut events: EventReader<MoveRequest>,
    window_index: Res<WindowIndex>,
    surface_query: Query<(Entity, &PhysicalRect)>,
    mut commands: Commands,
) {
    let Some(( _,pos ))=&output_focus.0 else{
        error!("cursor not found");
        return;
    };
    for MoveRequest(id) in &mut events {
        if let Some((entity, geo)) = window_index
            .get(id)
            .and_then(|e| surface_query.get(*e).ok())
        {
            commands.insert_resource(MovingState {
                relatice: pos.as_vec2() - point_to_vec2(geo.loc.to_f64()),
                backend: entity,
            });
            commands.insert_resource(NextState(Some(DWayClientState::Moving)));
        }
    }
}
pub fn move_window(
    moving_state: Res<MovingState>,
    mut cursor_move_events: EventReader<MouseMotion>,
    mut surface_query: Query<(&mut PhysicalRect, Option<&WindowScale>), With<WindowMark>>,
) {
    let Ok((mut rect,_window_scale))=surface_query.get_mut(moving_state.backend)else{
        error!(entity=?moving_state.backend,"window backend not found");
        return;
    };
    for event in cursor_move_events.iter() {
        let delta = ivec2_to_point(event.delta.as_ivec2());
        rect.loc += delta;
    }
}
pub fn stop_moving(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut commands: Commands,
) {
    for event in cursor_button_events.iter() {
        dbg!(event);
        if event.state == ButtonState::Released {
            commands.insert_resource(NextState::<DWayClientState>(None));
            commands.remove_resource::<MovingState>();
            return;
        }
    }
}
