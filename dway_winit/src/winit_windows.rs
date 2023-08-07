use std::sync::atomic::Ordering;

use accesskit_winit::Adapter;
use bevy::a11y::{
    accesskit::{NodeBuilder, NodeClassSet, Role, Tree, TreeUpdate},
    AccessKitEntityExt, AccessibilityRequested,
};
use bevy::ecs::entity::Entity;

use bevy::utils::{tracing::warn, HashMap};
use bevy::window::{CursorGrabMode, Window, WindowMode, WindowPosition, WindowResolution};
pub use bevy::winit::get_fitting_videomode;
pub use bevy::winit::get_best_videomode;

use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    monitor::MonitorHandle,
};

use crate::{
    accessibility::{AccessKitAdapters, WinitActionHandler, WinitActionHandlers},
    converters::convert_window_level,
};
pub use bevy::winit::WinitWindows;

pub(crate) fn attempt_grab(winit_window: &winit::window::Window, grab_mode: CursorGrabMode) {
    let grab_result = match grab_mode {
        bevy::window::CursorGrabMode::None => {
            winit_window.set_cursor_grab(winit::window::CursorGrabMode::None)
        }
        bevy::window::CursorGrabMode::Confined => winit_window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .or_else(|_e| winit_window.set_cursor_grab(winit::window::CursorGrabMode::Locked)),
        bevy::window::CursorGrabMode::Locked => winit_window
            .set_cursor_grab(winit::window::CursorGrabMode::Locked)
            .or_else(|_e| winit_window.set_cursor_grab(winit::window::CursorGrabMode::Confined)),
    };

    if let Err(err) = grab_result {
        let err_desc = match grab_mode {
            bevy::window::CursorGrabMode::Confined | bevy::window::CursorGrabMode::Locked => "grab",
            bevy::window::CursorGrabMode::None => "ungrab",
        };

        bevy::utils::tracing::error!("Unable to {} cursor: {}", err_desc, err);
    }
}

// Ideally we could generify this across window backends, but we only really have winit atm
// so whatever.
pub fn winit_window_position(
    position: &WindowPosition,
    resolution: &WindowResolution,
    mut available_monitors: impl Iterator<Item = MonitorHandle>,
    primary_monitor: Option<MonitorHandle>,
    current_monitor: Option<MonitorHandle>,
) -> Option<PhysicalPosition<i32>> {
    match position {
        WindowPosition::Automatic => {
            /* Window manager will handle position */
            None
        }
        WindowPosition::Centered(monitor_selection) => {
            use bevy::window::MonitorSelection::*;
            let maybe_monitor = match monitor_selection {
                Current => {
                    if current_monitor.is_none() {
                        warn!("Can't select current monitor on window creation or cannot find current monitor!");
                    }
                    current_monitor
                }
                Primary => primary_monitor,
                Index(n) => available_monitors.nth(*n),
            };

            if let Some(monitor) = maybe_monitor {
                let screen_size = monitor.size();

                let scale_factor = resolution.base_scale_factor();

                // Logical to physical window size
                let (width, height): (u32, u32) =
                    LogicalSize::new(resolution.width(), resolution.height())
                        .to_physical::<u32>(scale_factor)
                        .into();

                let position = PhysicalPosition {
                    x: screen_size.width.saturating_sub(width) as f64 / 2.
                        + monitor.position().x as f64,
                    y: screen_size.height.saturating_sub(height) as f64 / 2.
                        + monitor.position().y as f64,
                };

                Some(position.cast::<i32>())
            } else {
                warn!("Couldn't get monitor selected with: {monitor_selection:?}");
                None
            }
        }
        WindowPosition::At(position) => {
            Some(PhysicalPosition::new(position[0] as f64, position[1] as f64).cast::<i32>())
        }
    }
}

// WARNING: this only works under the assumption that wasm runtime is single threaded
#[cfg(target_arch = "wasm32")]
unsafe impl Send for WinitWindows {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for WinitWindows {}
