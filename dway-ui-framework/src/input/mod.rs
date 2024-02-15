use bevy_relationship::reexport::SmallVec;

use crate::prelude::*;

pub type Callback<E> = Option<(Entity, SystemId<E>)>;
pub type Callbacks<E> = SmallVec<[(Entity, SystemId<E>); 2]>;

#[derive(Resource, Default, Reflect)]
pub struct MousePosition {
    pub window: Option<Entity>,
    pub position: Option<Vec2>,
}

pub fn update_mouse_position(
    mut mouse_event: EventReader<CursorMoved>,
    mut mouse_position: ResMut<MousePosition>,
) {
    if let Some(mouse) = mouse_event.read().last() {
        mouse_position.window = Some(mouse.window);
        mouse_position.position = Some(mouse.position);
    }
}

structstruck::strike! {
    pub struct FocusEvent{
        pub receiver: Entity,
        pub node: Entity,
        pub event: pub enum FocusEventKind{
            MouseEnter,
            MouseLeave,
            KeybordEnter,
            KeyboardLeave,
        }
    }
}

#[derive(Component, Debug)]
pub struct UiFocus {
    pub callbacks: SmallVec<[(Entity, SystemId<FocusEvent>); 2]>,
    pub mouse_focused: bool,
    pub input_focused: bool,
}

pub struct UiFocusState {
    pub mouse_focus: Option<Entity>,
    pub input_focus: Option<Entity>,
}
