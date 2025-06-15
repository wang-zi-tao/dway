use wayland_server::protocol::wl_data_device_manager::DndAction;

use crate::prelude::*;

#[derive(Component, Reflect, Debug)]
pub struct DragIcon;

relationship!(DragAndDropRelationship=> DragTo -- DropFrom);

#[derive(Component, Reflect, Debug)]
pub struct DragAndDrop {
    pub data_source: Option<Entity>,
    pub origin_surface: Entity,
    pub icon_surface: Option<Entity>,
    pub serial: u32,
}

pub struct DropTarget {}

impl DragAndDrop {
    pub fn choise_action(available: DndAction, preferred: DndAction) -> DndAction {
        if [DndAction::Move, DndAction::Copy, DndAction::Ask].contains(&preferred)
            && available.contains(preferred)
        {
            preferred
        } else if available.contains(DndAction::Ask) {
            DndAction::Ask
        } else if available.contains(DndAction::Copy) {
            DndAction::Copy
        } else if available.contains(DndAction::Move) {
            DndAction::Move
        } else {
            DndAction::empty()
        }
    }
}
