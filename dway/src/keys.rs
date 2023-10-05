use bevy::{app::AppExit, input::mouse::MouseButtonInput, prelude::*};
use dway_client_core::desktop::{CursorOnWindow, FocusedWindow};
use dway_server::prelude::WindowAction;

pub fn wm_keys(
    input: Res<Input<KeyCode>>,
    mut window_under_cursor: Res<CursorOnWindow>,
    mut exit: EventWriter<AppExit>,
    mut window_action: EventWriter<WindowAction>,
) {
    let meta = input.any_pressed([KeyCode::LWin, KeyCode::RWin]);
    let shift = input.any_pressed([KeyCode::RShift, KeyCode::LShift]);
    let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);
    let alt = input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]);

    if alt | meta {
        if shift && input.just_pressed(KeyCode::Q) {
            exit.send(AppExit);
        } else if input.just_pressed(KeyCode::Q) || input.just_pressed(KeyCode::F4) {
            if let Some((window, _)) = &window_under_cursor.0 {
                window_action.send(WindowAction::Close(*window));
            }
        }
    }
}

pub fn wm_mouse_action(
    input: Res<Input<KeyCode>>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
) {
    let meta = input.any_pressed([KeyCode::LWin, KeyCode::RWin]);
    let shift = input.any_pressed([KeyCode::RShift, KeyCode::LShift]);
    let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);
    let alt = input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]);
}
