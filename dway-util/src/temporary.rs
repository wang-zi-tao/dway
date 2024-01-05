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
    temporary_tree_query.for_each_mut(|(entity, mut tmp)| {
        if !tmp.is_added() {
            if tmp.0 == 0 {
                commands.entity(entity).despawn_recursive();
            } else {
                tmp.0 -= 1;
            }
        }
    });
    temporary_entity_query.for_each_mut(|(entity, mut tmp)| {
        if !tmp.is_added() {
            if tmp.0 == 0 {
                commands.entity(entity).despawn();
            } else {
                tmp.0 -= 1;
            }
        }
    });
}
