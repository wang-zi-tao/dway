use bevy::{ecs::system::SystemId, ui::FocusPolicy};

use crate::{framework::button::UiButton, prelude::*};

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

pub enum PopupEvent {
    Opened,
    Closed,
}

#[derive(Component, Reflect, Default, Clone, Debug)]
pub struct UiPopup {
    pub close_policy: PopupClosePolicy,
    #[reflect(ignore)]
    pub callback: Option<SystemId>,
    pub state: PopupState,
    pub position: PopupPosition,
    pub moveable: bool,
    pub hovered: bool,
    pub anchor: Option<Entity>,
}

pub fn auto_close_popup(
    mut popup_query: Query<(&mut UiPopup, &Interaction)>,
    mouse: Res<Input<MouseButton>>,
    mut commands: Commands,
) {
    let mouse_down =
        || mouse.any_just_pressed([MouseButton::Left, MouseButton::Middle, MouseButton::Right]);
    popup_query.for_each_mut(|(mut popup, button_state)| {
        let mut run_callback = false;
        match popup.close_policy {
            PopupClosePolicy::MouseLeave => match button_state {
                Interaction::Pressed => {}
                Interaction::Hovered => {
                    popup.hovered = true;
                }
                Interaction::None => {
                    if popup.hovered || button_state == &Interaction::None && mouse_down() {
                        popup.state = PopupState::Closed;
                        run_callback = true;
                    }
                }
            },
            PopupClosePolicy::MouseButton => {
                if button_state == &Interaction::None && mouse_down() {
                    popup.state = PopupState::Closed;
                    run_callback = true;
                }
            }
            PopupClosePolicy::None => {}
        }
        if run_callback {
            if let Some(callback) = popup.callback {
                commands.run_system(callback);
            }
        }
    });
}

#[derive(Bundle, Default)]
pub struct UiPopupBundle {
    pub popup: UiPopup,

    pub button: UiButton,
    pub interaction: Interaction,

    pub node: Node,
    pub style: Style,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub z_index: ZIndex,
}

pub struct PopupUiPlugin;
impl Plugin for PopupUiPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiPopup>()
            .add_systems(Update, auto_close_popup);
    }
}
