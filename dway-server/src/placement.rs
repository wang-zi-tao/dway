use bevy::prelude::*;
use smithay::{desktop::Window, utils::Rectangle};

use crate::{
    components::*,
    events::{CreateTopLevelEvent, CreateWindow},
};

#[derive(Bundle)]
pub struct PlacementBundle {
    pub physical: PhysicalRect,
    pub global: GlobalPhysicalRect,
}

pub fn place_new_window(
    mut events: EventReader<CreateWindow>,
    window_index: Res<WindowIndex>,
    commands: ParallelCommands,
) {
    for e in events.iter() {
        if let Some(&entity) = window_index.0.get(&e.0) {
            let pos = (0, 0);
            let size = (800, 600);
            let physical = PhysicalRect(Rectangle::from_loc_and_size(pos, size));
            let global = GlobalPhysicalRect(Rectangle::from_loc_and_size(pos, size));
            commands.command_scope(move |mut c| {
                c.entity(entity)
                    .insert(PlacementBundle { global, physical });
            });
            info!("placement window on {entity:?}");
        }
    }
}

pub fn update_physical_rect(
    mut root_query: Query<
        (&mut PhysicalRect, &LogicalRect, Option<&WindowScale>),
        Or<(Changed<LogicalRect>, Changed<WindowScale>)>,
    >,
) {
    for (mut physical_rect, logical_rect, scale) in root_query.iter_mut() {
        physical_rect.0 = logical_rect
            .0
            .to_physical_precise_round(scale.cloned().unwrap_or_default().0);
    }
}

fn do_update_node(
    mut dest: Mut<GlobalPhysicalRect>,
    relative: PhysicalRect,
    mut context_rect: GlobalPhysicalRect,
    children: Option<&Children>,
    children_query: &Query<
        (&mut GlobalPhysicalRect, &PhysicalRect, Option<&Children>),
        With<Parent>,
    >,
) {
    context_rect.0.loc += relative.0.loc;
    *dest = context_rect;
    if let Some(c) = children {
        for child in c.iter() {
            if let Ok((global, relative, children)) =
                unsafe { children_query.get_unchecked(*child) }
            {
                do_update_node(global, *relative, context_rect, children, children_query);
            }
        }
    }
}
pub fn update_global_physical_rect(
    mut root_query: Query<
        (&mut GlobalPhysicalRect, &PhysicalRect, Option<&Children>),
        Without<Parent>,
    >,
    children_query: Query<
        (&mut GlobalPhysicalRect, &PhysicalRect, Option<&Children>),
        With<Parent>,
    >,
) {
    for (global, rect, children) in root_query.iter_mut() {
        do_update_node(global, *rect, Default::default(), children, &children_query);
    }
}
