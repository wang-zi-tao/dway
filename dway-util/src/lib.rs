use bevy::prelude::*;
use temporary::clean_temporary_entity;

pub mod asset_cache;
pub mod diagnostic;
pub mod eventloop;
pub mod formats;
pub mod keys;
pub mod logger;
pub mod macros;
pub mod render;
pub mod stat;
pub mod temporary;
pub mod tokio;
mod typed_ecs;

#[cfg(feature = "debug")]
pub mod debug;

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
