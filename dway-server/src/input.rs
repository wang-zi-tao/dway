use bevy::prelude::*;

use crate::{
    components::{WindowIndex, WindowMark, WlSurfaceWrapper},
    events::MouseMoveOnWindow,
};

pub fn on_mouse_move(
    mut events: EventReader<MouseMoveOnWindow>,
    window_index: Res<WindowIndex>,
    surface_query: Query<(&WlSurfaceWrapper), With<WindowMark>>,
) {
    for MouseMoveOnWindow(id, position) in events.iter() {
        if let Some(surface) = window_index
            .get(id)
            .and_then(|&e| surface_query.get(e).ok())
        {}
    }
}

pub fn on_mouse_button() {}
