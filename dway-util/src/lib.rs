use bevy::prelude::*;
use temporary::clean_temporary_entity;

pub mod eventloop;
pub mod logger;
pub mod macros;
pub mod stat;
pub mod temporary;
pub mod keys;
pub mod tokio;
pub mod formats;
mod typed_ecs;
pub mod render;

pub struct UtilPlugin;
impl Plugin for UtilPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        if !app.is_plugin_added::<Self>() {
            app.add_systems(First, clean_temporary_entity);
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}

