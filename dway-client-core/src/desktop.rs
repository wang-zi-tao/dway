use std::num::NonZeroUsize;

use bevy::prelude::*;
use lru::LruCache;


use crate::DWayClientSystem;

pub struct DWayDesktop;
impl Plugin for DWayDesktop {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(WindowStack::default());
        app.insert_resource(FocusedWindow::default());
        app.insert_resource(CursorOnOutput::default());
        app.insert_resource(WindowHistory::default());
        app.add_system(update_window_stack_by_focus.in_set(DWayClientSystem::UpdateState));
        app.add_system(update_z_index.in_set(DWayClientSystem::UpdateState));
        app.register_type::<FocusedWindow>();
        app.register_type::<CursorOnOutput>();
    }
}

#[derive(Resource)]
pub struct WindowStack(pub LruCache<Entity, ()>);
impl Default for WindowStack {
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(65535).unwrap()))
    }
}

#[derive(Resource, Default, Reflect)]
pub struct FocusedWindow(pub Option<Entity>);

#[derive(Resource, Default, Reflect)]
pub struct CursorOnOutput(pub Option<(Entity, IVec2)>);

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
            window_stack.0.push(*focused_window, ());
        }
    }
}
pub fn update_z_index(
    window_stack: Res<WindowStack>,
    mut window_meta_query: Query<&mut Transform>,
) {
    if !window_stack.is_changed() {
        return;
    }
    for (i, (&window_entity, ())) in window_stack.0.iter().enumerate() {
        if let Ok(mut transform) = window_meta_query.get_mut(window_entity) {
            transform.translation.z = 256.0 - (i as f32);
        }
    }
}
