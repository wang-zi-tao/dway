use bevy::{
    prelude::*,
    window::{InternalWindowState, PresentMode, WindowLevel, WindowMode, WindowResolution},
};

use crate::drm::{connectors::Connector, surface::DrmSurface};

pub fn create_window(conn: &Connector, surface: &DrmSurface) -> Window {
    let size = surface.size();
    Window {
        present_mode: PresentMode::AutoVsync,
        mode: WindowMode::Fullscreen,
        position: WindowPosition::At(IVec2::default()),
        resolution: WindowResolution::new(size.x as f32, size.y as f32),
        title: conn.name.clone(),
        composite_alpha_mode: bevy::window::CompositeAlphaMode::Opaque,
        resize_constraints: WindowResizeConstraints {
            min_width: size.x as f32,
            min_height: size.y as f32,
            max_width: size.x as f32,
            max_height: size.y as f32,
        },
        resizable: false,
        decorations: false,
        transparent: false,
        focused: false,
        window_level: WindowLevel::AlwaysOnTop,
        canvas: None,
        fit_canvas_to_parent: false,
        prevent_default_event_handling: false,
        ..Default::default()
    }
}

pub fn relative_to_window(window: &Window, pos: Vec2) -> Option<Vec2> {
    let WindowPosition::At(window_position) = window.position else {
        return None;
    };
    let window_size = Vec2::new(window.resolution.width(), window.resolution.height());
    let relative = pos - window_position.as_vec2();
    if relative.x < 0.0
        || relative.x >= window_size.x
        || relative.y < 0.0
        || relative.x >= window_size.y
    {
        return None;
    }
    Some(relative)
}
