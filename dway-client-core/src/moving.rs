use std::time::SystemTime;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
        ButtonState,
    },
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use dway_protocol::window::{WindowMessage, WindowMessageKind};

use crate::{
    desktop::{CursorOnOutput, FocusedWindow},
    protocol::WindowMessageSender,
    stages::DWayStage,
    window::WindowMetadata,
};
#[derive(Resource,Default)]
pub struct MoveRelative(Vec2);
#[derive(Default)]
pub struct DWayMovingPlugin {}
impl Plugin for DWayMovingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MoveRelative::default());
        app.add_system_set(SystemSet::on_enter(DWayStage::Moving).with_system(start_moving));
        app.add_system_set(
            SystemSet::on_update(DWayStage::Moving)
                .with_system(move_window)
                .with_system(stop_moving),
        );
    }
}
pub fn start_moving(
    focused_window: Res<FocusedWindow>,
    windows: Query<&WindowMetadata>,
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
    let Ok(  meta )=windows.get(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    move_relative.0 = pos.as_vec2() - meta.geo.min;
}
pub fn move_window(
    focused_window: Res<FocusedWindow>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut windows: Query<(&mut WindowMetadata, &mut Style)>,
    sender: Res<WindowMessageSender>,
    physical_windows: Res<Windows>,
    move_relative: Res<MoveRelative>,
    mut output_focus: ResMut<CursorOnOutput>,
) {
    if cursor_move_events.is_empty() {
        return;
    }
    let Some(focus_window)=&focused_window.0 else{
        return;
    };
    let Ok( ( mut meta,mut style ) )=windows.get_mut(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    for event in cursor_move_events.iter() {
        let Some( window )=physical_windows.get(event.id)else{
            error!("failed to get window {}",event.id);
            continue;
        };
        let pos: Vec2 =
            Vec2::new(event.position.x, window.height() - event.position.y) - move_relative.0;
        output_focus.0 = Some((event.id, pos.as_ivec2()));
        crate::window::move_window(&mut meta, &mut style, pos);
        if let Err(e) = sender.0.send(WindowMessage {
            uuid: meta.uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::Move(pos.as_ivec2()),
        }) {
            error!("failed to send message: {}", e);
            continue;
        };
    }
}
pub fn stop_moving(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut status: ResMut<State<DWayStage>>,
    mut move_relative: ResMut<MoveRelative>,
) {
    if cursor_button_events.is_empty() {
        return;
    }
    for event in cursor_button_events.iter() {
        if event.state == ButtonState::Released {
            if let Err(e) = status.pop() {
                error!("failed to enter moving stage: {}", e);
            };
            move_relative.0 = Vec2::default();
            return;
        }
    }
}
