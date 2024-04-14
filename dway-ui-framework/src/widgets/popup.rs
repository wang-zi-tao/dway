use crate::{
    animation::ui::{
        despawn_recursive_on_animation_finish, popup_open_close_up, popup_open_drop_down,
        UiAnimationConfig,
    },
    make_bundle,
    prelude::*,
};
use bevy::ui::RelativeCursorPosition;
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
    pub receiver: Entity,
    pub kind: PopupEventKind,
}

structstruck::strike! {
    #[derive(Component, Reflect, SmartDefault, Clone, Debug, Builder)]
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
        pub callbacks: CallbackSlots<PopupEvent>,
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
        #[default(true)]
        pub(crate) mouse_state_init: bool,
        pub auto_destroy: bool,
        pub anchor: Option<Entity>,
    }
}

impl UiPopup {
    pub fn with_callback(mut self, receiver: Entity, callback: SystemId<PopupEvent, ()>) -> Self {
        self.callbacks.push((receiver, callback));
        self
    }
    pub fn with_auto_destroy(mut self) -> Self {
        self.auto_destroy = true;
        self
    }
}

pub fn update_popup(
    mut popup_query: Query<(Entity, &mut UiPopup, &RelativeCursorPosition)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    for (entity, mut popup, relative_cursor) in popup_query.iter_mut() {
        let run_close_callback =
            |popup: &UiPopup, commands: &mut Commands, kind: PopupEventKind| {
                for (receiver, callback) in &popup.callbacks {
                    commands.run_system_with_input(
                        *callback,
                        PopupEvent {
                            entity,
                            kind,
                            receiver: *receiver,
                        },
                    );
                }
            };
        let mouse_inside = relative_cursor.mouse_over();
        if popup.is_added() && popup.state == PopupState::Open {
            run_close_callback(&popup, &mut commands, PopupEventKind::Opened);
        }
        match popup.close_policy {
            PopupClosePolicy::MouseLeave => {
                if !mouse_inside {
                    if !popup.hovered {
                        popup.state = PopupState::Closed;
                        run_close_callback(&popup, &mut commands, PopupEventKind::Closed);
                    }
                } else {
                    popup.hovered = true;
                }
            }
            PopupClosePolicy::MouseButton => {
                if mouse_down() {
                    if !mouse_inside && !popup.mouse_state_init{
                        popup.state = PopupState::Closed;
                        run_close_callback(&popup, &mut commands, PopupEventKind::Closed);
                    }
                } else {
                    if popup.mouse_state_init {
                        popup.mouse_state_init = false;
                    }
                }
            }
            PopupClosePolicy::None => {}
        }
        if popup.state == PopupState::Closed && popup.auto_destroy {
            commands.entity(entity).despawn_recursive();
        };
    }
}

make_bundle! {
    @from popup: UiPopup,
    @addon UiPopupExt,
    UiPopupBundle {
        pub popup: UiPopup,
        pub relative_cursor: RelativeCursorPosition,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
    }
}

pub fn popup_animation_system<C: UiAnimationConfig>(
    In(event): In<PopupEvent>,
    theme: Res<Theme>,
    mut commands: Commands,
) {
    match event.kind {
        PopupEventKind::Opened => {
            commands.entity(event.entity).insert(
                Animation::new(C::appear_time(), C::appear_ease())
                    .with_callback(C::appear_animation(&theme)),
            );
        }
        PopupEventKind::Closed => {
            commands.entity(event.entity).insert(
                Animation::new(C::disappear_time(), C::disappear_ease())
                    .with_callback(C::disappear_animation(&theme)),
            );
        }
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
