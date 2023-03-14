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
    pub surface_offset: SurfaceOffset,
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
            let surface_offset = SurfaceOffset(Rectangle::from_loc_and_size((0, 0), size));
            commands.command_scope(move |mut c| {
                c.entity(entity).insert(PlacementBundle {
                    global,
                    physical,
                    surface_offset,
                });
            });
            info!("placement window on {entity:?}");
        }
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
    context_rect.loc += relative.loc;
    *dest = context_rect;
    dest.size = relative.size;
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
pub fn update_logical_rect(
    mut rect_query: Query<
        (&mut LogicalRect, &PhysicalRect, Option<&WindowScale>),
        Or<(Changed<PhysicalRect>, Changed<WindowScale>)>,
    >,
) {
    for (mut logical, physical, scale) in &mut rect_query {
        let scale = scale.cloned().unwrap_or_default().0;
        let new_logical = physical.to_f64().to_logical(scale).to_i32_round();
        if new_logical != logical.0 {
            logical.0 = new_logical;
        }
    }
}
