use std::time::SystemTime;

use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseMotion},
        ButtonState,
    },
    prelude::*,
    utils::tracing,
    winit::WinitWindows,
};
use dway_server::{events::MoveRequest, geometry::GlobalGeometry};

use crate::{
    desktop::{CursorOnOutput, FocusedWindow},
    window::Backend,
    DWayClientState, DWayClientSystem,
};
#[derive(Resource)]
pub struct MovingState {
    relatice: IVec2,
    backend: Entity,
}
#[derive(Default)]
pub struct DWayMovingPlugin {}
impl Plugin for DWayMovingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(start_moving.run_if(on_event::<MoveRequest>()));
        app.add_system(
            move_window
                .run_if(
                    in_state(DWayClientState::Moving)
                        .and_then(on_event::<CursorMoved>())
                        .and_then(resource_exists::<MovingState>()),
                )
                .in_set(OnUpdate(DWayClientState::Moving)),
        );
        app.add_system(
            stop_moving
                .run_if(in_state(DWayClientState::Moving).and_then(on_event::<MouseButtonInput>()))
                .after(move_window),
        );
    }
}
// #[tracing::instrument(skip_all)]
pub fn start_moving(
    output_focus: Res<CursorOnOutput>,
    mut events: EventReader<MoveRequest>,
    surface_query: Query<&GlobalGeometry>,
    mut commands: Commands,
) {
    let Some(( _,pos ))=&output_focus.0 else{
        error!("cursor not found");
        return;
    };
    for MoveRequest(entity) in &mut events {
        if let Ok(geo) = surface_query.get(*entity) {
            commands.insert_resource(MovingState {
                relatice: *pos - geo.geometry.pos(),
                backend: *entity,
            });
            commands.insert_resource(NextState(Some(DWayClientState::Moving)));
            trace!("start moving");
        }
    }
}
// #[tracing::instrument(skip_all)]
pub fn move_window(
    output_focus: Res<CursorOnOutput>,
    moving_state: Res<MovingState>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut surface_query: Query<(&mut GlobalGeometry), With<GlobalGeometry>>,
) {
    let Some(( _,pos ))=&output_focus.0 else{
        error!("cursor not found");
        return;
    };
    let Ok((mut rect))=surface_query.get_mut(moving_state.backend)else{
        error!(entity=?moving_state.backend,"window backend not found");
        return;
    };
    for event in cursor_move_events.iter() {
        // rect.loc = ivec2_to_point(*pos - moving_state.relatice);
    }
}
// #[tracing::instrument(skip_all)]
pub fn stop_moving(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut commands: Commands,
) {
    for event in cursor_button_events.iter() {
        if event.state == ButtonState::Released {
            trace!("stop moving");
            commands.insert_resource(NextState::<DWayClientState>(Some(DWayClientState::Desktop)));
            commands.remove_resource::<MovingState>();
            return;
        }
    }
}
