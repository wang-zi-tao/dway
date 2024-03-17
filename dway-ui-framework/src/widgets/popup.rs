use crate::{make_bundle, prelude::*};
use bevy::{ecs::system::SystemId, ui::{FocusPolicy, RelativeCursorPosition}};
use derive_builder::Builder;
use interpolation::EaseFunction;

#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq)]
pub enum PopupEventKind {
    Opened,
    Closed,
}
#[derive(Debug)]
pub struct PopupEvent {
    pub entity: Entity,
    pub kind: PopupEventKind,
}

structstruck::strike!{
    #[derive(Component, Reflect, Default, Clone, Debug, Builder)]
    pub struct UiPopup {
        pub close_policy:
            #[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq)]
            pub enum PopupClosePolicy {
                #[default]
                MouseButton,
                MouseLeave,
                None,
            },
        #[reflect(ignore)]
        pub callback: Option<SystemId<PopupEvent>>,
        pub state: 
            #[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq)]
            pub enum PopupState {
                #[default]
                Open,
                Closed,
            },
        pub position: 
            #[derive(Default, Clone, Copy, Reflect, Debug)]
            pub enum PopupPosition {
                Up,
                #[default]
                Down,
                Left,
                Right,
            },
        pub moveable: bool,
        pub hovered: bool,
        pub auto_destroy: bool,
        pub anchor: Option<Entity>,
    }
}

impl UiPopup {
    pub fn new(callback: Option<SystemId<PopupEvent, ()>>) -> Self {
        UiPopup {
            callback,
            ..default()
        }
    }
    pub fn new_auto_destroy(callback: Option<SystemId<PopupEvent, ()>>) -> Self {
        UiPopup {
            callback,
            auto_destroy: true,
            ..default()
        }
    }
}

pub fn update_popup(
    mut popup_query: Query<(Entity, &mut UiPopup, &Node, &GlobalTransform, &RelativeCursorPosition)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    popup_query.for_each_mut(|(entity, mut popup, node, transform, relative_cursor)| {
        let run_close_callback = |popup: &UiPopup, commands: &mut Commands| {
            if let Some(callback) = popup.callback {
                commands.run_system_with_input(
                    callback,
                    PopupEvent {
                        entity,
                        kind: PopupEventKind::Closed,
                    },
                );
            }
        };
        let slider_rect = Rect::from_center_size(transform.translation().xy(), node.size());
        let mouse_inside = relative_cursor.mouse_over();
        match popup.close_policy {
            PopupClosePolicy::MouseLeave => {
                if !mouse_inside {
                    if !popup.hovered {
                        popup.state = PopupState::Closed;
                        run_close_callback(&popup, &mut commands);
                    }
                } else {
                    popup.hovered = true;
                }
            }
            PopupClosePolicy::MouseButton => {
                if !mouse_inside && mouse_down() {
                    popup.state = PopupState::Closed;
                    run_close_callback(&popup, &mut commands);
                }
            }
            PopupClosePolicy::None => {}
        }
        if popup.state == PopupState::Closed && popup.auto_destroy {
            commands.entity(entity).despawn_recursive();
        };
    });
}

make_bundle!{
    @from popup: UiPopup,
    @addon UiPopupExt,
    UiPopupBundle {
        pub popup: UiPopup,
        pub relative_cursor: RelativeCursorPosition,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
    }
}

pub fn delay_destroy(In(event): In<PopupEvent>, mut commands: Commands) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).despawn_recursive(); // TODO: temp code
        // commands.entity(event.entity).insert(Animation::new(Duration::from_secs_f32(0.4), EaseFunction::BackOut).with_callback(theme.system(AnimationSystem::default())));
    }
}

pub fn delay_destroy_up(In(event): In<PopupEvent>, mut commands: Commands) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).despawn_recursive(); // TODO: temp code
        // commands.entity(event.entity).insert(despawn_animation(
        //     animation!(Tween 0.5 secs:BackOut->TransformPositionLens(Vec3::NEG_Y=>Vec3::Y)),
        // ));
    }
}
