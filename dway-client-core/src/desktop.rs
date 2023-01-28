use std::{collections::HashMap, num::NonZeroUsize};

use bevy::{prelude::{Entity, Resource, IVec2, Plugin}, window::WindowId};
use lru::LruCache;
use uuid::Uuid;

pub struct DWayDesktop;
impl Plugin for DWayDesktop{
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(WindowSet::default());
        app.insert_resource(FocusedWindow::default());
        app.insert_resource(CursorOnOutput::default());
        app.insert_resource(WindowHistory::default());
    }
}

#[derive(Resource, Default)]
pub struct WindowSet {
    pub window_set: HashMap<Uuid, Entity>,
}

#[derive(Resource, Default)]
pub struct FocusedWindow(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct CursorOnOutput(pub Option<(WindowId,IVec2)>);

#[derive(Resource)]
pub struct WindowHistory(pub LruCache<Uuid,Entity>);
impl Default for WindowHistory{
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(255).unwrap()))
    }
}
