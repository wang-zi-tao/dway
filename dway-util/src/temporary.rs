use bevy::prelude::*;

#[derive(Component)]
pub struct TemporaryEntity;

#[derive(Component)]
pub struct TemporaryTree;

pub fn clean_temporary_entity(
    temporary_tree_query: Query<(Entity, Ref<TemporaryTree>)>,
    temporary_entity_query: Query<(Entity, Ref<TemporaryEntity>), Without<TemporaryTree>>,
    mut commands: Commands,
) {
    temporary_tree_query.for_each(|(entity, tmp)| {
        if !tmp.is_added() {
            commands.entity(entity).despawn_recursive()
        }
    });
    temporary_entity_query.for_each(|(entity, tmp)| {
        if !tmp.is_added() {
            commands.entity(entity).despawn()
        }
    });
}
