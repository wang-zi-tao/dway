use bevy::{
    ecs::system::EntityCommands,
    ui::RelativeCursorPosition,
};
use derive_builder::Builder;

use crate::{
    animation::{
        ui::UiAnimationConfig,
        AnimationEvent,
    },
    event::{make_callback, EventReceiver, UiNodeAppearEvent},
    make_bundle,
    prelude::*,
    theme::{ThemeComponent, WidgetKind},
};

#[derive(Resource, Default)]
pub struct PopupStack {
    pub stack: Vec<Entity>,
}

#[derive(Debug, Clone, Copy, Reflect, PartialEq, Eq)]
pub enum UiPopupEvent {
    Opened,
    Closed,
}

pub type UiPopupEventDispatcher = EventDispatcher<UiPopupEvent>;

impl<T: EventReceiver<UiNodeAppearEvent>> EventReceiver<UiPopupEvent> for T {
    fn on_event(&self, commands: EntityCommands, event: UiPopupEvent) {
        let appear_event = match &event {
            UiPopupEvent::Opened => UiNodeAppearEvent::Appear,
            UiPopupEvent::Closed => UiNodeAppearEvent::Disappear,
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
        &UiPopupEventDispatcher,
    )>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    for (entity, mut popup, relative_cursor, event_dispatcher) in popup_query.iter_mut() {
        let mouse_inside = relative_cursor.mouse_over();
        if popup.is_added() && popup.state == PopupState::Open {
            event_dispatcher.send(UiPopupEvent::Opened, &mut commands);
        }
        if popup.state == PopupState::Open {
            if popup.request_close {
                popup.state = PopupState::Closed;
                event_dispatcher.send(UiPopupEvent::Closed, &mut commands);
            } else {
                match popup.close_policy {
                    PopupClosePolicy::MouseLeave => {
                        if !mouse_inside {
                            if !popup.hovered {
                                popup.state = PopupState::Closed;
                                event_dispatcher.send(UiPopupEvent::Closed, &mut commands);
                            }
                        } else {
                            popup.hovered = true;
                        }
                    }
                    PopupClosePolicy::MouseButton => {
                        if mouse_down() {
                            if !mouse_inside && !popup.mouse_state_init {
                                popup.state = PopupState::Closed;
                                event_dispatcher.send(UiPopupEvent::Closed, &mut commands);
                            }
                        } else if popup.mouse_state_init {
                            popup.mouse_state_init = false;
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
        pub event_dispatcher: UiPopupEventDispatcher,
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
    event: UiEvent<UiPopupEvent>,
    callbacks: Res<CallbackTypeRegister>,
    mut commands: Commands,
) {
    match &*event {
        UiPopupEvent::Opened => {
            commands.entity(event.receiver()).insert((
                Animation::new(C::appear_time(), C::appear_ease()),
                make_callback(event.sender(), C::appear_animation(&callbacks)),
            ));
        }
        UiPopupEvent::Closed => {
            commands.entity(event.receiver()).insert((
                Animation::new(C::disappear_time(), C::disappear_ease()),
                make_callback(event.sender(), C::disappear_animation(&callbacks)),
            ));
        }
    }
}

pub fn delay_destroy(event: UiEvent<UiPopupEvent>, mut commands: Commands) {
    if matches!(&*event, UiPopupEvent::Closed) {
        commands.entity(event.receiver()).despawn_recursive();
    }
}
