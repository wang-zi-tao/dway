use std::f32::consts::PI;

use bevy::{
    app::AppExit,
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
};
use dway_client_core::{
    desktop::{CursorOnOutput, CursorOnWindow, FocusedWindow},
    navigation::windowstack::{WindowIndex, WindowStack},
    workspace::{ScreenAttachWorkspace, WindowOnWorkspace, Workspace, WorkspaceSet},
};
use dway_server::{
    apps::launchapp::{RunCommandRequest, RunCommandRequestBuilder},
    input::grab::ResizeEdges,
    macros::EntityCommandsExt,
    prelude::WindowAction,
};
use dway_ui::{framework::gallary::WidgetGallaryBundle, prelude::spawn, widgets::popup::UiPopupAddonBundle};

pub fn wm_keys(
    input: Res<Input<KeyCode>>,
    window_under_cursor: Res<CursorOnWindow>,
    mut exit: EventWriter<AppExit>,
    mut window_action: EventWriter<WindowAction>,
    window_stack: Res<WindowStack>,
    focus_screen: Res<CursorOnOutput>,
    mut focus_window: ResMut<FocusedWindow>,
    workspace_root_query: Query<&Children, With<WorkspaceSet>>,
    mut commands: Commands,
    mut tab_counter: Local<usize>,
    mut run_command_event: EventWriter<RunCommandRequest>,
) {
    let meta = input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let shift = input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

    if alt | meta {
        if input.just_pressed(KeyCode::Return) {
            run_command_event.send(
                RunCommandRequestBuilder::default()
                    .command("alacritty".to_string())
                    .build()
                    .unwrap(),
            )
        } else if shift && input.just_pressed(KeyCode::Q) {
            exit.send(AppExit);
        } else if input.just_pressed(KeyCode::Q) || input.just_pressed(KeyCode::F4) {
            if let Some((window, _)) = &window_under_cursor.0 {
                window_action.send(WindowAction::Close(*window));
            }
        } else if input.just_pressed(KeyCode::Tab) {
            *tab_counter += 1;
            if let Some(window) = &window_stack.at(*tab_counter) {
                focus_window.window_entity = Some(*window);
            }
        } else {
            for (key, num) in [
                (KeyCode::Key1, 0),
                (KeyCode::Key2, 1),
                (KeyCode::Key3, 2),
                (KeyCode::Key4, 3),
                (KeyCode::Key5, 4),
                (KeyCode::Key6, 5),
                (KeyCode::Key7, 6),
                (KeyCode::Key8, 7),
                (KeyCode::Key9, 8),
                (KeyCode::Key0, 9),
            ] {
                if input.just_pressed(key) {
                    match (meta, shift, ctrl, alt) {
                        (true, false, false, false) => {
                            if let Ok(workspaces) = workspace_root_query.get_single() {
                                if let (Some(workspace), Some((screen, _))) =
                                    (workspaces.get(num), &focus_screen.0)
                                {
                                    commands
                                        .entity(*screen)
                                        .disconnect_all::<ScreenAttachWorkspace>()
                                        .connect_to::<ScreenAttachWorkspace>(*workspace);
                                }
                            }
                        }
                        (true, false, true, false) => {
                            if let Ok(workspaces) = workspace_root_query.get_single() {
                                if let (Some(workspace), Some((screen, _))) =
                                    (workspaces.get(num), &focus_screen.0)
                                {
                                    commands
                                        .entity(*screen)
                                        .connect_to::<ScreenAttachWorkspace>(*workspace);
                                }
                            }
                        }
                        (true, true, false, false) => {
                            if let Ok(workspaces) = workspace_root_query.get_single() {
                                if let (Some(workspace), Some(window)) =
                                    (workspaces.get(num), &focus_window.window_entity)
                                {
                                    commands
                                        .entity(*window)
                                        .disconnect_all::<WindowOnWorkspace>()
                                        .connect_to::<WindowOnWorkspace>(*workspace);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if alt && input.just_pressed(KeyCode::F5) {
        spawn! {
            &mut commands=>
            <WidgetGallaryBundle @style="absolute right-32 bottom-32" UiPopupAddonBundle=(Default::default()) />
        }
    }

    if input.just_released(KeyCode::SuperRight) || input.just_released(KeyCode::SuperLeft) {
        *tab_counter = 0;
    }
    if input.just_released(KeyCode::AltLeft) || input.just_released(KeyCode::AltRight) {
        *tab_counter = 0;
    }
}

pub fn wm_mouse_action(
    keys: Res<Input<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mouse_buttons: Res<Input<MouseButton>>,
    focused_window: Res<FocusedWindow>,
    mut window_action: EventWriter<WindowAction>,
    _mouse_button_events: EventReader<MouseButtonInput>,
    mut mouse_drag_delta: Local<Option<Vec2>>,
) {
    let meta = keys.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let _shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let _ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);
    let mouse_down = mouse_buttons.get_pressed().next().is_some();
    let mouse_just_down = mouse_buttons.get_just_pressed().next().is_some();
    let _mouse_just_up = mouse_buttons.get_just_released().next().is_some();

    if mouse_just_down {
        *mouse_drag_delta = Some(Vec2::ZERO);
    }
    for motion in mouse_motion.read() {
        if let Some(delta) = mouse_drag_delta.as_mut() {
            *delta = *delta + motion.delta;
        }
    }
    if !mouse_down {
        *mouse_drag_delta = None;
    }

    if meta | alt {
        if let Some(focused_window_entity) = focused_window.window_entity {
            if mouse_buttons.just_pressed(MouseButton::Left) {
                window_action.send(WindowAction::RequestMove(focused_window_entity));
            } else if mouse_buttons.pressed(MouseButton::Right) {
                if let Some(delta) = mouse_drag_delta.as_mut() {
                    if delta.length_squared() >= 64.0 {
                        let direction = delta.normalize();
                        let threshold = (PI * 3.0 / 8.0).cos();
                        let edges = (if direction.x > threshold {
                            ResizeEdges::RIGHT
                        } else if direction.x < -threshold {
                            ResizeEdges::LEFT
                        } else {
                            ResizeEdges::default()
                        }) | (if direction.y > threshold {
                            ResizeEdges::BUTTOM
                        } else if direction.y < -threshold {
                            ResizeEdges::TOP
                        } else {
                            ResizeEdges::default()
                        });
                        if !edges.is_empty() {
                            window_action
                                .send(WindowAction::RequestResize(focused_window_entity, edges));
                        }
                        *mouse_drag_delta = None;
                    }
                }
            }
        }
    }
}
