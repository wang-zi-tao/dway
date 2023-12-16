use bevy::input::mouse::MouseMotion;

use crate::prelude::*;

#[derive(Component, Default, Debug, Reflect)]
pub struct Draggable {
    #[reflect(ignore)]
    pub callback: Option<(Entity, SystemId<DraggableEvent>)>,
}

#[derive(Debug, Reflect)]
pub enum DraggableEventKind {
    Move(Vec2),
}

#[derive(Event, Debug, Reflect)]
pub struct DraggableEvent {
    pub receiver: Entity,
    pub entity: Entity,
    pub kind: DraggableEventKind,
}

pub fn update_draggable(
    graggable_query: Query<(Entity, &Draggable, &Interaction)>,
    mut mouse_event: EventReader<MouseMotion>,
    mut commands: Commands,
) {
    graggable_query.for_each(|(entity, draggable, intersection)| {
        if *intersection != Interaction::Pressed {
            return;
        }
        let delta = mouse_event.read().fold(Vec2::ZERO, |d, m| d + m.delta);
        if let Some((receiver, callback)) = &draggable.callback {
            commands.run_system_with_input(
                *callback,
                DraggableEvent {
                    receiver: *receiver,
                    entity,
                    kind: DraggableEventKind::Move(delta),
                },
            );
        };
    });
}

#[derive(Bundle, Default)]
pub struct DraggableAddonBundle {
    pub draggable: Draggable,
    pub interaction: Interaction,
}

impl DraggableAddonBundle {
    pub fn new(draggable: Draggable) -> Self {
        Self {
            draggable,
            interaction: Default::default(),
        }
    }
}
