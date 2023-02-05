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
    window::{set_window_rect, WindowMetadata},
};

#[derive(Resource, Default)]
pub struct ResizingMethod {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

#[derive(Default)]
pub struct DWayResizingPlugin {}
impl Plugin for DWayResizingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ResizingMethod::default());
        app.add_system_set(
            SystemSet::on_update(DWayStage::Resizing)
                .with_system(resize_window)
                .with_system(stop_resizing),
        );
    }
}
pub fn resize_window(
    focused_window: Res<FocusedWindow>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut windows: Query<&mut WindowMetadata>,
    sender: Res<WindowMessageSender>,
    physical_windows: Res<Windows>,
    resize_method: Res<ResizingMethod>,
    mut output_focus: ResMut<CursorOnOutput>,
) {
    if cursor_move_events.is_empty() {
        return;
    }
    let Some(focus_window)=&focused_window.0 else{
        return;
    };
    let Ok( mut meta )=windows.get_mut(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    for event in cursor_move_events.iter() {
        let Some( window )=physical_windows.get(event.id)else{
            error!("failed to get window {}",event.id);
            continue;
        };
        let pos: Vec2 = Vec2::new(event.position.x, window.height() - event.position.y);
        output_focus.0 = Some((event.id, pos.as_ivec2()));
        let mut geo = meta.geo;
        if resize_method.top {
            geo.min.y = pos.y;
        }
        if resize_method.bottom {
            geo.max.y = pos.y;
        }
        if resize_method.left {
            geo.min.x = pos.x;
        }
        if resize_method.right {
            geo.max.x = pos.x;
        }
        set_window_rect(&mut meta, geo);
        if let Err(e) = sender.0.send(WindowMessage {
            uuid: meta.uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::SetRect(geo),
        }) {
            error!("failed to send message: {}", e);
            continue;
        };
    }
}
pub fn stop_resizing(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut status: ResMut<State<DWayStage>>,
    mut resize_method: ResMut<ResizingMethod>,
) {
    if cursor_button_events.is_empty() {
        return;
    }
    for event in cursor_button_events.iter() {
        if event.state == ButtonState::Released {
            if let Err(e) = status.pop() {
                error!("failed to leave resizing stage: {}", e);
            };
            *resize_method = ResizingMethod::default();
            return;
        }
    }
}
