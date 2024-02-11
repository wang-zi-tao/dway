use bevy::{ecs::system::SystemId, ui::FocusPolicy};
use derive_builder::Builder;

use crate::{
    // animation,
    framework::{
        button::{self, UiButton},
        MousePosition,
    },
    prelude::*,
};

#[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq)]
pub enum PopupState {
    #[default]
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq)]
pub enum PopupClosePolicy {
    #[default]
    MouseButton,
    MouseLeave,
    None,
}

#[derive(Default, Clone, Copy, Reflect, Debug)]
pub enum PopupPosition {
    Up,
    #[default]
    Down,
    Left,
    Right,
}

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

#[derive(Component, Reflect, Default, Clone, Debug, Builder)]
pub struct UiPopup {
    pub close_policy: PopupClosePolicy,
    #[reflect(ignore)]
    pub callback: Option<SystemId<PopupEvent>>,
    pub state: PopupState,
    pub position: PopupPosition,
    pub moveable: bool,
    pub hovered: bool,
    pub auto_destroy: bool,
    pub anchor: Option<Entity>,
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

pub fn auto_close_popup(
    mut popup_query: Query<(Entity, &mut UiPopup, &Node, &GlobalTransform)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_position: Res<'_, MousePosition>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    popup_query.for_each_mut(|(entity, mut popup, node, transform)| {
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
        let mouse_inside = slider_rect.contains(mouse_position.position.unwrap_or_default());
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

#[derive(Bundle, SmartDefault)]
pub struct UiPopupAddonBundle {
    pub popup: UiPopup,

    pub button: UiButton,
    pub interaction: Interaction,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
}
impl From<UiPopup> for UiPopupAddonBundle {
    fn from(value: UiPopup) -> Self {
        Self {
            popup: value,
            ..Default::default()
        }
    }
}

#[derive(Bundle, SmartDefault)]
pub struct UiPopupBundle {
    pub popup: UiPopup,

    pub button: UiButton,
    pub interaction: Interaction,

    pub node: Node,
    pub style: Style,
    #[default(FocusPolicy::Block)]
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum PopupUiSystems {
    Close,
}

pub fn delay_destroy(In(event): In<PopupEvent>, mut commands: Commands) {
    if PopupEventKind::Closed == event.kind {
        commands.entity(event.entity).despawn_recursive(); // TODO: temp code
        // commands.entity(event.entity).insert(despawn_animation(
        //     animation!(Tween 0.5 secs:BackIn->TransformScaleLens(Vec3::ONE=>Vec3::splat(0.5))),
        // ));
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

pub struct PopupUiPlugin;
impl Plugin for PopupUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiPopup>()
            .register_system(delay_destroy)
            .register_system(delay_destroy_up)
            .add_systems(
                Update,
                auto_close_popup
                    .in_set(PopupUiSystems::Close)
                    .after(button::process_ui_button_event),
            );
    }
}
