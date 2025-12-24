use bevy::prelude::*;

#[derive(Component, Default)]
pub struct TemporaryEntity(pub usize);

#[derive(Component, Default)]
pub struct TemporaryTree(pub usize);

pub fn clean_temporary_entity(
    mut temporary_tree_query: Query<(Entity, &mut TemporaryTree)>,
    mut temporary_entity_query: Query<(Entity, &mut TemporaryEntity), Without<TemporaryTree>>,
    mut commands: Commands,
) {
    for (entity, mut tmp) in temporary_tree_query.iter_mut() {
        if !tmp.is_added() {
            if tmp.0 == 0 {
                commands.entity(entity).despawn();
            } else {
                tmp.0 -= 1;
            }
        }
    }
    for (entity, mut tmp) in temporary_entity_query.iter_mut() {
        if !tmp.is_added() {
            if tmp.0 == 0 {
                commands.entity(entity).despawn();
            } else {
                tmp.0 -= 1;
            }
        }
    }
}
