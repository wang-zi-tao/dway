use std::time::SystemTime;

use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    winit::WinitWindows,
};
use dway_protocol::window::{WindowMessage, WindowMessageKind};
use dway_server::{
    components::{GlobalPhysicalRect, PhysicalRect, WindowMark, WindowScale},
    math::{ivec2_to_point, point_to_vec2, vec2_to_point},
};

use crate::{
    desktop::{CursorOnOutput, FocusedWindow},
    protocol::WindowMessageSender,
    window::Backend,
    DWayClientState, DWayClientSystem,
};
#[derive(Resource, Default)]
pub struct MoveRelative(Vec2);
#[derive(Default)]
pub struct DWayMovingPlugin {}
impl Plugin for DWayMovingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MoveRelative::default());
        app.add_system(start_moving.in_schedule(OnEnter(DWayClientState::Moving)));
        app.add_system(move_window.in_set(OnUpdate(DWayClientState::Moving)));
        app.add_system(start_moving.in_set(OnUpdate(DWayClientState::Moving)));
    }
}
pub fn start_moving(
    focused_window: Res<FocusedWindow>,
    windows: Query<&Backend>,
    surface_query: Query<&GlobalPhysicalRect>,
    mut move_relative: ResMut<MoveRelative>,
    output_focus: Res<CursorOnOutput>,
) {
    let Some(( _,pos ))=&output_focus.0 else{
        error!("cursor not found");
        return;
    };
    let Some(focus_window)=&focused_window.0 else{
        return;
    };
    let Ok(  backend )=windows.get(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    let Ok(  geo )=surface_query.get(backend.get())else {
        error!("window backend entity {focus_window:?} not found");
        return;
    };
    move_relative.0 = pos.as_vec2() - point_to_vec2(geo.loc.to_f64());
}
pub fn move_window(
    focused_window: Res<FocusedWindow>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut window_query: Query<(&mut Backend, &mut Style)>,
    mut surface_query: Query<(&mut PhysicalRect, Option<&WindowScale>), With<WindowMark>>,
    sender: Res<WindowMessageSender>,
    move_relative: Res<MoveRelative>,
    mut output_focus: ResMut<CursorOnOutput>,
) {
    if cursor_move_events.is_empty() {
        return;
    }
    let Some(focus_window)=&focused_window.0 else{
        return;
    };
    let Ok( ( backend,style ) )=window_query.get_mut(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    let Ok((mut rect,window_scale))=surface_query.get_mut(backend.get())else{
        error!("window backend entity {focus_window:?} not found");
        return;
    };
    let scale = window_scale.cloned().unwrap_or_default().0;
    for event in cursor_move_events.iter() {
        let delta = ivec2_to_point(event.position.as_ivec2());
        rect.loc += delta;
    }
}
pub fn stop_moving(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut stages: ResMut<State<DWayClientState>>,
    mut commands: Commands,
) {
    for event in cursor_button_events.iter() {
        if event.state == ButtonState::Released {
            commands.insert_resource(NextState(Some(DWayClientState::Desktop)));
            return;
        }
    }
}
