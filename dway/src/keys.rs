use bevy::{app::AppExit, prelude::*, input::mouse::MouseButtonInput};

pub fn wm_keys(input: Res<Input<KeyCode>>, mut exit: EventWriter<AppExit>) {
    let meta = input.any_pressed([KeyCode::LWin, KeyCode::RWin]);
    let shift = input.any_pressed([KeyCode::RShift, KeyCode::LShift]);
    let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);
    let alt = input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]);

    if meta && input.just_pressed(KeyCode::Q) {
        exit.send(AppExit);
    }
}

pub fn wm_mouse_action(input: Res<Input<KeyCode>>, mut mouse_button_events: EventReader<MouseButtonInput>) {
    let meta = input.any_pressed([KeyCode::LWin, KeyCode::RWin]);
    let shift = input.any_pressed([KeyCode::RShift, KeyCode::LShift]);
    let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);
    let alt = input.any_pressed([KeyCode::LAlt, KeyCode::RAlt]);


}
