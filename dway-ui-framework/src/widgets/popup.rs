use bevy::{ecs::system::EntityCommands, ui::RelativeCursorPosition};
use derive_builder::Builder;

use crate::{
    animation::{ui::UiAnimationConfig, AnimationEvent},
    event::{make_callback, EventReceiver, UiNodeAppearEvent},
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
    #[require(Node, RelativeCursorPosition, UiPopupEventDispatcher)]
    #[require(FocusPolicy=FocusPolicy::Block)]
    #[require(ThemeComponent=ThemeComponent::widget(WidgetKind::BlurBackground))]
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

    pub fn with_close_policy(mut self, policy: PopupClosePolicy) -> Self {
        self.close_policy = policy;
        self
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
        let mouse_inside = relative_cursor.cursor_over;
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
                commands.entity(entity).despawn();
            };
        }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
#[require(Node)]
#[relationship_target(relationship = AttachToAnchor, linked_spawn)]
pub struct Anchor(Vec<Entity>);

#[derive(Component, Reflect)]
#[relationship(relationship_target = Anchor)]
pub struct AttachToAnchor(pub Entity);

#[derive(Reflect, Clone, Debug)]
pub enum PopupAnlign {
    None,
    CameraStart,
    OutterStart,
    InnerStart,
    Center,
    InnerEnd,
    OutterEnd,
    CameraEnd,
}

#[derive(Component, SmartDefault, Reflect, Clone, Debug)]
pub struct AnchorPolicy {
    #[default(PopupAnlign::Center)]
    pub horizontal_align: PopupAnlign,
    #[default(PopupAnlign::OutterEnd)]
    pub vertical_align: PopupAnlign,
}

impl AnchorPolicy {
    pub fn new(horizontal_align: PopupAnlign, vertical_align: PopupAnlign) -> Self {
        Self {
            vertical_align,
            horizontal_align,
        }
    }
}

pub fn anchor_update_system(
    mut popup_query: Query<
        (
            &mut UiTargetCamera,
            &AnchorPolicy,
            &mut Node,
            Option<&mut AnimationTargetNodeState>,
        ),
        With<UiPopup>,
    >,
    anchor_query: Query<
        (
            Ref<Anchor>,
            Ref<ComputedUiTargetCamera>,
            Ref<ComputedNode>,
            Ref<GlobalTransform>,
        ),
        (
            With<Anchor>,
            Or<(
                Changed<Anchor>,
                Changed<ComputedNode>,
                Changed<ComputedUiTargetCamera>,
                Changed<Anchor>,
            )>,
        ),
    >,
    camera_query: Query<&Camera>,
) {
    for (anchor, anchor_target, anchor_node, anchor_transform) in anchor_query.iter() {
        let anchor_rect = Rect::from_center_size(
            anchor_transform.translation().truncate(),
            anchor_node.size(),
        );

        let mut iter = popup_query.iter_many_mut(&*anchor.0);
        while let Some((mut popup_camera, anchor_policy, mut popup_node, mut animation_end_state)) =
            iter.fetch_next()
        {
            let node = if let Some(state) = animation_end_state.as_deref_mut() {
                &mut state.0
            } else {
                &mut popup_node
            };

            if let Some(camera) = anchor_target.get() {
                popup_camera.0 = camera;
            }
            let Ok(camera) = camera_query.get(popup_camera.0) else {
                continue;
            };

            let Some(camera_size) = camera.logical_viewport_size() else {
                continue;
            };

            let compute_on_direction = |align: PopupAnlign,
                                        camera_len: f32,
                                        anchor_start: f32,
                                        anchor_end: f32,
                                        node_start: &mut Val,
                                        node_end: &mut Val| {
                match align {
                    PopupAnlign::None => {}
                    PopupAnlign::CameraStart => {
                        *node_start = Val::Percent(0.0);
                    }
                    PopupAnlign::OutterStart => {
                        *node_start = Val::Px(anchor_end);
                    }
                    PopupAnlign::InnerStart => {
                        *node_start = Val::Px(anchor_start);
                    }
                    PopupAnlign::Center => {}
                    PopupAnlign::InnerEnd => {
                        *node_end = Val::Px(camera_len - anchor_end);
                    }
                    PopupAnlign::OutterEnd => {
                        *node_end = Val::Px(camera_len - anchor_start);
                    }
                    PopupAnlign::CameraEnd => {
                        *node_end = Val::Percent(0.0);
                    }
                }
            };

            compute_on_direction(
                anchor_policy.horizontal_align.clone(),
                camera_size.x,
                anchor_rect.min.x,
                anchor_rect.max.x,
                &mut node.left,
                &mut node.right,
            );

            compute_on_direction(
                anchor_policy.vertical_align.clone(),
                camera_size.y,
                anchor_rect.min.y,
                anchor_rect.max.y,
                &mut node.top,
                &mut node.bottom,
            );
        }
    }
}

impl EventReceiver<AnimationEvent> for UiPopup {
    fn on_event(&self, mut commands: EntityCommands, event: AnimationEvent) {
        if self.state == PopupState::Closed && event.just_finish {
            commands.despawn();
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
        commands.entity(event.receiver()).despawn();
    }
}
