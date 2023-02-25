use bevy_ecs::prelude::*;
use smithay::{desktop::Window, utils::Rectangle};

use crate::{
    components::*, events::{CreateTopLevelEvent, CreateWindow},
};

pub fn place_new_window(
    mut events: EventReader<CreateWindow>,
    window_index: Res<WindowIndex>,
    commands: ParallelCommands,
) {
    for e in events.iter() {
        if let Some(&entity) = window_index.0.get(&e.0) {
            let rect = PhysicalRect(Rectangle::from_loc_and_size((0, 0), (800, 600)));
            commands.command_scope(move |mut c| {
                c.entity(entity).insert(rect);
            });
        }
    }
}

