use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
};
use smithay::{
    input::pointer::{ButtonEvent, MotionEvent},
    utils::{Point, SERIAL_COUNTER},
};

use crate::{
    components::{WaylandWindow, WindowIndex, WindowMark, WlSurfaceWrapper},
    events::{MouseButtonOnWindow, MouseMoveOnWindow},
    seat::PointerFocus,
    DWayServerComponent,
};

pub fn on_mouse_move(
    time: Res<Time>,
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseMoveOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<&WaylandWindow, With<WindowMark>>,
) {
    let dway = &mut dway_query.single_mut().dway;
    for MouseMoveOnWindow(id, pos) in events.iter() {
        if let Some(window) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let serial = SERIAL_COUNTER.next_serial();
            let point = Point::from((pos.x as f64, pos.y as f64));
            dway.seat.get_pointer().unwrap().motion(
                dway,
                Some((
                    PointerFocus::WaylandWindow(window.0.clone()),
                    Default::default(),
                )),
                &MotionEvent {
                    location: point,
                    serial,
                    time: time.elapsed().as_millis() as u32,
                },
            );
        }
    }
}

pub fn on_mouse_button(
    time: Res<Time>,
    mut dway_query: Query<&mut DWayServerComponent>,
    mut events: EventReader<MouseButtonOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<&WaylandWindow, With<WindowMark>>,
) {
    let dway = &mut dway_query.single_mut().dway;
    for MouseButtonOnWindow(id, pos, MouseButtonInput { button, state }) in events.iter() {
        if let Some(window) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {
            let serial = SERIAL_COUNTER.next_serial();
            dway.seat.get_pointer().unwrap().button(
                dway,
                &ButtonEvent {
                    serial,
                    time: time.elapsed().as_millis() as u32,
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
