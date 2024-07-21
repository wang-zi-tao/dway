use std::f32::consts::PI;

use bevy::{
    app::AppExit,
    input::mouse::{MouseButtonInput, MouseMotion},
    prelude::*,
    winit::WinitSettings,
};
use bevy_relationship::{graph_query2, ControlFlow};
use dway_client_core::{
    desktop::{CursorOnScreen, CursorOnWindow, FocusedWindow},
    layout::tile::{TileLayoutKind, TileLayoutSet},
    navigation::windowstack::WindowStack,
    workspace::{
        ScreenAttachWorkspace, ScreenWorkspaceList, WindowOnWorkspace, WorkspaceManager,
        WorkspaceRequest, WorkspaceSet,
    },
};
use dway_server::{
    apps::launchapp::{RunCommandRequest, RunCommandRequestBuilder},
    input::grab::ResizeEdges,
    macros::{graph_query, EntityCommandsExt},
    prelude::WindowAction,
    xdg::toplevel::DWayToplevel,
};

graph_query2! {
WmGraph=>
    mut screen_workspace=match
        (screen:Entity)-[ScreenAttachWorkspace]->
            (workspace:(Entity,Option<(&mut TileLayoutKind, &mut TileLayoutSet)>));
}

pub fn wm_keys(
    mut graph: WmGraph,
    input: Res<ButtonInput<KeyCode>>,
    window_under_cursor: Res<CursorOnWindow>,
    mut exit: EventWriter<AppExit>,
    mut window_action: EventWriter<WindowAction>,
    window_stack: Res<WindowStack>,
    focus_screen: Res<CursorOnScreen>,
    mut focus_window: ResMut<FocusedWindow>,
    mut commands: Commands,
    mut tab_counter: Local<usize>,
    mut run_command_event: EventWriter<RunCommandRequest>,
    workspace_manager: Res<WorkspaceManager>,
    window_query: Query<&DWayToplevel>,
    maybe_winit: Option<Res<WinitSettings>>,
) {
    let mut meta = input.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
    let shift = input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let ctrl = input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let alt = input.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

    if maybe_winit.is_some() {
        meta = alt;
    }

    if meta {
        if input.just_pressed(KeyCode::Enter) {
            run_command_event.send(
                RunCommandRequestBuilder::default()
                    .command("tilix".to_string())
                    .build()
                    .unwrap(),
            );
        } else if input.just_pressed(KeyCode::Space) {
            if let Some((screen, _)) = &focus_screen.0 {
                graph.foreach_screen_workspace_mut_from(*screen, |_, (_, tile)| {
                    if let Some((tile_kind, tile_set)) = tile {
                        **tile_kind = tile_set.add_index(1).clone();
                    }
                    ControlFlow::<()>::Continue
                });
            }
        } else if input.just_pressed(KeyCode::F11) {
            if let Some((window, _)) = &window_under_cursor.0 {
                if let Ok(toplevel) = window_query.get(*window) {
                    if toplevel.fullscreen {
                        window_action.send(WindowAction::UnFullscreen(*window));
                    } else {
                        window_action.send(WindowAction::Fullscreen(*window));
                    }
                }
            }
        } else if input.just_pressed(KeyCode::KeyM) {
            if let Some((window, _)) = &window_under_cursor.0 {
                if let Ok(toplevel) = window_query.get(*window) {
                    if toplevel.max {
                        window_action.send(WindowAction::UnMaximize(*window));
                    } else {
                        window_action.send(WindowAction::Maximize(*window));
                    }
                }
            }
        } else if input.just_pressed(KeyCode::KeyH) {
            if let Some((window, _)) = &window_under_cursor.0 {
                if let Ok(toplevel) = window_query.get(*window) {
                    if toplevel.min {
                        window_action.send(WindowAction::UnMinimize(*window));
                    } else {
                        window_action.send(WindowAction::Minimize(*window));
                    }
                }
            }
        } else if shift && input.just_pressed(KeyCode::KeyQ) {
            exit.send(AppExit::Success);
        } else if input.just_pressed(KeyCode::KeyQ) || input.just_pressed(KeyCode::F4) {
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
                (KeyCode::Digit1, 0),
                (KeyCode::Digit2, 1),
                (KeyCode::Digit3, 2),
                (KeyCode::Digit4, 3),
                (KeyCode::Digit5, 4),
                (KeyCode::Digit6, 5),
                (KeyCode::Digit7, 6),
                (KeyCode::Digit8, 7),
                (KeyCode::Digit9, 8),
                (KeyCode::Digit0, 9),
            ] {
                if let (Some(workspace), Some((screen, _))) =
                    (workspace_manager.workspaces.get(num), &focus_screen.0)
                {
                    if input.just_pressed(key) {
                        match (shift, ctrl, alt) {
                            (false, false, _) => commands.trigger_targets(
                                WorkspaceRequest::AttachToScreen {
                                    screen: *screen,
                                    unique: true,
                                },
                                *workspace,
                            ),
                            (false, true, _) => commands.trigger_targets(
                                WorkspaceRequest::AttachToScreen {
                                    screen: *screen,
                                    unique: false,
                                },
                                *workspace,
                            ),
                            (true, false, _) => {
                                if let Some(window) = &focus_window.window_entity {
                                    commands.trigger_targets(
                                        WorkspaceRequest::AttachWindow {
                                            window: *window,
                                            unique: true,
                                        },
                                        *workspace,
                                    )
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        };
    }

    if input.just_released(KeyCode::SuperRight) || input.just_released(KeyCode::SuperLeft) {
        *tab_counter = 0;
    }
    if input.just_released(KeyCode::AltLeft) || input.just_released(KeyCode::AltRight) {
        *tab_counter = 0;
    }
}

pub fn wm_mouse_action(
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
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
            *delta += motion.delta;
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
                            *mouse_drag_delta = None;
                        }
                    }
                }
            }
        }
    }
}
