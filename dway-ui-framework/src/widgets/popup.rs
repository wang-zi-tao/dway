use bevy::{ecs::system::EntityCommands, ui::RelativeCursorPosition};
use derive_builder::Builder;
use interpolation::EaseFunction;

use crate::{
    animation::{
        ui::{
            despawn_recursive_on_animation_finish, popup_open_close_up, popup_open_drop_down,
            UiAnimationConfig,
        },
        AnimationEvent,
    },
    event::{EventReceiver, UiNodeAppearEvent},
    make_bundle,
    prelude::*,
    render::layer_manager::RenderToLayer,
    theme::{ThemeComponent, WidgetKind},
};

#[derive(Resource, Default)]
pub struct PopupStack {
    pub stack: Vec<Entity>,
}

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

impl<T: EventReceiver<UiNodeAppearEvent>> EventReceiver<PopupEvent> for T {
    fn on_event(&self, commands: EntityCommands, event: PopupEvent) {
        let appear_event = match event.kind {
            PopupEventKind::Opened => UiNodeAppearEvent::Appear,
            PopupEventKind::Closed => UiNodeAppearEvent::Disappear,
        };
        self.on_event(commands, appear_event);
    }
}

structstruck::strike! {
    #[derive(Component, Reflect, SmartDefault, Clone, Debug, Builder)]
    #[builder(default)]
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
        pub request_close: bool,
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

    pub fn request_close(&mut self) {
        self.request_close = true;
    }
}

pub fn update_popup(
    mut popup_query: Query<(
        Entity,
        &mut UiPopup,
        &RelativeCursorPosition,
        Option<&dyn EventReceiver<PopupEvent>>,
    )>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    for (entity, mut popup, relative_cursor, dispatchs) in popup_query.iter_mut() {
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
                let mut entity_commands = commands.entity(entity);
                for dispatch in dispatchs.iter().flatten() {
                    dispatch.on_event(
                        entity_commands.reborrow(),
                        PopupEvent {
                            entity,
                            kind,
                            receiver: entity,
                        },
                    );
                }
            };
        let mouse_inside = relative_cursor.mouse_over();
        if popup.is_added() && popup.state == PopupState::Open {
            run_close_callback(&popup, &mut commands, PopupEventKind::Opened);
        }
        if popup.state == PopupState::Open {
            if popup.request_close {
                popup.state = PopupState::Closed;
                run_close_callback(&popup, &mut commands, PopupEventKind::Closed);
            } else {
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
                            if !mouse_inside && !popup.mouse_state_init {
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
            }
            if popup.state == PopupState::Closed && popup.auto_destroy {
                commands.entity(entity).despawn_recursive();
            };
        }
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
        #[default(ThemeComponent::widget(WidgetKind::BlurBackground))]
        pub theme: ThemeComponent,
    }
}

impl EventReceiver<AnimationEvent> for UiPopup {
    fn on_event(&self, commands: EntityCommands, event: AnimationEvent) {
        if self.state == PopupState::Closed && event.just_finish {
            commands.despawn_recursive();
        }
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
