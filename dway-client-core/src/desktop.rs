use std::{collections::HashMap, num::NonZeroUsize};

use bevy::{prelude::*, window::WindowId};
use lru::LruCache;
use uuid::Uuid;

use crate::window::WindowMetadata;

pub struct DWayDesktop;
impl Plugin for DWayDesktop {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(WindowSet::default());
        app.insert_resource(WindowStack::default());
        app.insert_resource(FocusedWindow::default());
        app.insert_resource(CursorOnOutput::default());
        app.insert_resource(WindowHistory::default());
        app.add_system(update_window_stack_by_focus);
        app.add_system(update_z_index);
    }
}

#[derive(Resource, Default)]
pub struct WindowSet {
    pub window_set: HashMap<Uuid, Entity>,
}
#[derive(Resource)]
pub struct WindowStack(pub LruCache<Entity, ()>);
impl Default for WindowStack {
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(65535).unwrap()))
    }
}

#[derive(Resource, Default)]
pub struct FocusedWindow(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct CursorOnOutput(pub Option<(WindowId, IVec2)>);

#[derive(Resource)]
pub struct WindowHistory(pub LruCache<Entity, ()>);
impl Default for WindowHistory {
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(65535).unwrap()))
    }
}

pub fn update_window_stack_by_focus(
    window_focus: Res<FocusedWindow>,
    mut window_stack: ResMut<WindowStack>,
) {
    if window_focus.is_changed() {
        if let Some(focused_window) = window_focus.0.as_ref() {
            window_stack.0.push(*focused_window,());
        }
    }
}
pub fn update_z_index(
    window_stack: Res<WindowStack>,
    mut window_meta_query: Query<(&WindowMetadata, &mut ZIndex,&mut Transform)>,
) {
    if !window_stack.is_changed() {
        return;
    }
    for (i, (&window_entity, ())) in window_stack.0.iter().enumerate() {
        if let Ok((window, mut z_index,mut transform)) = window_meta_query.get_mut(window_entity) {
            // *z_index = ZIndex::Global(65536-(i as i32));
            // *z_index = ZIndex::Local(0);
            transform.translation.z=256.0-( i as f32) ;
            // transform.rotation=Default::default();
            // transform.rotate_local_y(1.0);
        }
    }
}
