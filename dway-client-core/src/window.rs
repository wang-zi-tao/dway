use bevy::prelude::*;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Hidden;

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, _app: &mut App) {
    }
}
