use std::time::{Instant, SystemTime};

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
use bevy_mod_picking::{PickingEvent, PickingRaycastSet};

use bevy_mod_raycast::Intersection;
use log::info;

use dway_protocol::window::{WindowMessage, WindowMessageKind};

use crate::stages::DWayStage;

use super::{
    desktop::{CursorOnOutput, FocusedWindow},
    protocol::WindowMessageSender,
    window::WindowMetadata,
};

#[derive(Default)]
pub struct DWayInputPlugin {
    pub debug: bool,
}
impl Plugin for DWayInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(CoreStage::PreUpdate, print_pick_events);
        // app.add_system_to_stage(DWayStage::Desktop, mouse_move_on_window);
        app.add_system_set(
            SystemSet::on_update(DWayStage::Desktop).with_system(mouse_move_on_window),
        );
        app.add_system_to_stage(CoreStage::PreUpdate, mouse_button_on_window);
        app.add_system_to_stage(CoreStage::PreUpdate, keyboard_input_system);
        if self.debug {
            app.add_startup_system(setup_debug_cursor);
            app.add_system(debug_follow_cursor);
        }
    }
}
#[derive(Component)]
pub struct DebugCursor;
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
pub fn debug_follow_cursor(
    mut cursor_moved_events: EventReader<CursorMoved>,
    windows: Res<Windows>,
    mut cursor: Query<&mut Style, With<DebugCursor>>,
) {
    for event in cursor_moved_events.iter() {
        let Some( window )=windows.get(event.id)else{
            error!("failed to get window {}",event.id);
            continue;
        };
        let pos: Vec2 = (event.position.x, window.height() - event.position.y).into();
        let mut cursor = cursor.single_mut();
        cursor.position = UiRect {
            left: Val::Px(pos.x),
            top: Val::Px(pos.y),
            ..default()
        };
    }
}

pub fn print_pick_events(
    mut events: EventReader<PickingEvent>,
    mut cursors: Query<(Entity, &Intersection<PickingRaycastSet>)>,
) {
    for event in events.iter() {
        match event {
            PickingEvent::Selection(e) => info!("A selection event happened: {:?}", e),
            PickingEvent::Hover(e) => info!("Egads! A hover event!? {:?}", e),
            PickingEvent::Clicked(e) => info!(
                "Gee Willikers, it's a click! {:?} at {:?}",
                e,
                cursors.single().1.position()
            ),
        }
    }
}
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
pub fn keyboard_input_system(
    sender: Res<WindowMessageSender>,
    mut keyboard_evens: EventReader<KeyboardInput>,
    output_focus: Res<FocusedWindow>,
    windows: Query<&WindowMetadata>,
) {
    if keyboard_evens.is_empty() {
        return;
    }
    let Some(focus_window)=&output_focus.0 else{
        return;
    };
    let Ok( meta )=windows.get(*focus_window)else {
        error!("window entity {focus_window:?} not found");
        return;
    };
    for event in keyboard_evens.iter() {
        if let Err(e) = sender.0.send(WindowMessage {
            uuid: meta.uuid,
            time: SystemTime::now(),
            data: WindowMessageKind::KeyboardInput(*event),
        }) {
            error!("failed to send message: {}", e);
        };
    }
}
pub fn mouse_move_on_window(
    mut cursor_moved_events: EventReader<CursorMoved>,
    sender: Res<WindowMessageSender>,
    windows: Res<Windows>,
    elements: Query<(Entity, &WindowMetadata, &ZIndex)>,
    mut output_focus: ResMut<CursorOnOutput>,
) {
    for event in cursor_moved_events.iter() {
        let Some( window )=windows.get(event.id)else{
            error!("failed to get window {}",event.id);
            continue;
        };
        let pos: Vec2 = (event.position.x, window.height() - event.position.y).into();
        output_focus.0 = Some((event.id, pos.as_ivec2()));
        let min_z_element = window_unser_position(pos, &elements);
        if let Some((element, meta, z_index)) = min_z_element {
            if let Err(e) = sender.0.send(WindowMessage {
                uuid: meta.uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::MouseMove(pos - meta.geo.min),
            }) {
                error!("failed to send message: {}", e);
            };
        }
    }
}
fn mouse_button_on_window(
    mut cursor_button_events: EventReader<MouseButtonInput>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut sender: Res<WindowMessageSender>,
    mut windows: Res<Windows>,
    mut elements: Query<(Entity, &WindowMetadata, &ZIndex)>,
    output_focus: Res<CursorOnOutput>,
    mut focus: ResMut<FocusedWindow>,
) {
    if cursor_button_events.is_empty() && mouse_wheel_events.is_empty() {
        return;
    }
    let Some(( window_id,pos ))=&output_focus.0 else{
        return;
    };
    let Some( window )=windows.get(*window_id)else{
        error!("failed to get window {}",window_id);
        return;
    };
    let pos = pos.as_vec2();
    let min_z_element = window_unser_position(pos, &elements);
    if let Some((element, meta, z_index)) = min_z_element {
        focus.0 = Some(element);
        for event in cursor_button_events.iter() {
            if let Err(e) = sender.0.send(WindowMessage {
                uuid: meta.uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::MouseButton(*event),
            }) {
                error!("failed to send message: {}", e);
            };
        }
        for event in mouse_wheel_events.iter() {
            if let Err(e) = sender.0.send(WindowMessage {
                uuid: meta.uuid,
                time: SystemTime::now(),
                data: WindowMessageKind::MouseWheel(MouseWheel {
                    unit: event.unit,
                    x: event.x * 4.0,
                    y: -event.y * 4.0,
                }),
            }) {
                error!("failed to send message: {}", e);
            };
        }
    }
}
fn window_unser_position<'f>(
    pos: Vec2,
    elements: &'f Query<(Entity, &WindowMetadata, &ZIndex)>,
) -> Option<(Entity, &'f WindowMetadata, &'f ZIndex)> {
    let mut min_z = None;
    let mut min_z_element = None;
    for (element, meta, z_index) in elements.iter() {
        if meta.geo.contains(pos) {
            let update_min_z = if let Some(min_z_value) = &min_z {
                match (min_z_value, &z_index) {
                    (ZIndex::Local(_), ZIndex::Global(z)) if z <= &0 => true,
                    (ZIndex::Global(m), ZIndex::Global(z)) if z >= m => true,
                    (ZIndex::Local(m), ZIndex::Local(z)) if z >= m => true,
                    _ => false,
                }
            } else {
                true
            };
            if update_min_z {
                min_z = Some(*z_index);
                min_z_element = Some((element, meta, z_index));
            }
        }
    }
    min_z_element
}
