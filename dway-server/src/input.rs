use std::time::{SystemTime, UNIX_EPOCH};

use bevy::{
    core::NonSendMarker,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
        ButtonState,
    },
    prelude::*,
};
use smithay::{
    input::{
        keyboard::{FilterResult, KeyboardTarget},
        pointer::{ButtonEvent, MotionEvent, PointerTarget, RelativeMotionEvent},
    },
    utils::{Point, SERIAL_COUNTER},
};

use crate::{
    components::{
        SurfaceOffset, WaylandWindow, WindowIndex, WindowMark, WindowScale, WlSurfaceWrapper,
    },
    events::{KeyboardInputOnWindow, MouseButtonOnWindow, MouseMoveOnWindow, MouseWheelOnWindow, MouseMotionOnWindow},
    seat::PointerFocus,
    DWayServerComponent, EventLoopResource,
};

pub fn on_mouse_move(
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseMoveOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<
        (
            &WlSurfaceWrapper,
            Option<&SurfaceOffset>,
            Option<&WindowScale>,
        ),
        With<WindowMark>,
    >,
) {
    if events.is_empty() {
        return;
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    let dway = &mut dway_query.single_mut().dway;
    for MouseMoveOnWindow(id, pos) in events.iter() {
        if let Some((surface, offset, scale)) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let scale = scale.cloned().unwrap_or_default().0;
            let offset = offset
                .cloned()
                .unwrap_or_default()
                .0
                .loc
                .to_f64()
                .to_logical(scale)
                .to_i32_round();
            let serial = SERIAL_COUNTER.next_serial();
            let point = Point::from((pos.x as f64, pos.y as f64));
            if let Some(ptr) = dway.seat.get_pointer() {
                ptr.motion(
                    dway,
                    Some((surface.0.clone(), offset)),
                    &MotionEvent {
                        location: point,
                        serial,
                        time,
                    },
                );
            }
        }
    }
}
pub fn on_mouse_motion(
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseMotionOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<
        (
            &WlSurfaceWrapper,
            Option<&SurfaceOffset>,
            Option<&WindowScale>,
        ),
        With<WindowMark>,
    >,
) {
    if events.is_empty() {
        return;
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    let dway = &mut dway_query.single_mut().dway;
    for MouseMotionOnWindow(id, delta) in events.iter() {
        if let Some((surface, offset, scale)) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let scale = scale.cloned().unwrap_or_default().0;
            let offset = offset
                .cloned()
                .unwrap_or_default()
                .0
                .loc
                .to_f64()
                .to_logical(scale)
                .to_i32_round();
            let serial = SERIAL_COUNTER.next_serial();
            let delta = Point::from((delta.x as f64, delta.y as f64));
            if let Some(ptr) = dway.seat.get_pointer() {
                ptr.relative_motion(
                    dway,
                    Some((surface.0.clone(), offset)),
                    &RelativeMotionEvent {
                        delta: delta,
                        delta_unaccel: delta,
                        utime: time as u64,
                    },
                );
            }
        }
    }
}

pub fn on_mouse_button(
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseButtonOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<&WlSurfaceWrapper, With<WindowMark>>,
) {
    if events.is_empty() {
        return;
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    let dway = &mut dway_query.single_mut().dway;
    for MouseButtonOnWindow(id, pos, MouseButtonInput { button, state }) in events.iter() {
        if let Some(surface) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let serial = SERIAL_COUNTER.next_serial();
            surface.button(
                &dway.seat.clone(),
                dway,
                &ButtonEvent {
                    serial,
                    time,
                    button: match button {
                        MouseButton::Left => 0x110,
                        MouseButton::Right => 0x111,
                        MouseButton::Middle => 0x112,
                        MouseButton::Other(o) => *o as u32,
                    },
                    state: match state {
                        ButtonState::Pressed => smithay::backend::input::ButtonState::Pressed,
                        ButtonState::Released => smithay::backend::input::ButtonState::Released,
                    },
                },
            );
        }
    }
}
pub fn on_mouse_wheel(
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseWheelOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<&WlSurfaceWrapper, With<WindowMark>>,
) {
    if events.is_empty() {
        return;
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    let dway = &mut dway_query.single_mut().dway;
    for MouseWheelOnWindow(id, pos, MouseWheel { unit, x, y }) in events.iter().cloned() {
        if let Some(surface) = window_index
            .get(&id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            surface.axis(
                &dway.seat.clone(),
                dway,
                smithay::input::pointer::AxisFrame {
                    source: None,
                    time,
                    axis: ((x * 4.0) as f64, (y * 4.0) as f64),
                    discrete: match unit {
                        MouseScrollUnit::Line => None,
                        MouseScrollUnit::Pixel => Some((x as i32, y as i32)),
                    },
                    stop: (false, false),
                },
            );
        }
    }
}
pub fn on_keyboard(
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<KeyboardInputOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<&WlSurfaceWrapper, With<WindowMark>>,
) {
    if events.is_empty() {
        return;
    }
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;
    let dway = &mut dway_query.single_mut().dway;
    for KeyboardInputOnWindow(
        id,
        KeyboardInput {
            scan_code,
            key_code,
            state,
        },
    ) in events.iter()
    {
        dbg!(*scan_code, key_code, state);
        if let Some(surface) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let serial = SERIAL_COUNTER.next_serial();
            let keyboard = dway.seat.get_keyboard().unwrap();
            keyboard.set_focus(dway, Some(surface.0.clone()), serial);
            keyboard.input(
                dway,
                // key_code as u32,
                *scan_code,
                match state {
                    ButtonState::Pressed => smithay::backend::input::KeyState::Pressed,
                    ButtonState::Released => smithay::backend::input::KeyState::Released,
                },
                serial,
                time,
                |_, _, _| FilterResult::<()>::Forward,
            );
        }
    }
}
