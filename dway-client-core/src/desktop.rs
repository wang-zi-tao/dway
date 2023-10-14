use std::num::NonZeroUsize;

use bevy::prelude::*;
use lru::LruCache;


use crate::DWayClientSystem;

pub struct DWayDesktop;
impl Plugin for DWayDesktop {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(FocusStack::default());
        app.insert_resource(FocusedWindow::default());
        app.insert_resource(CursorOnOutput::default());
        app.insert_resource(CursorOnWindow::default());
        app.add_system(update_window_stack_by_focus.in_set(DWayClientSystem::UpdateState));
        app.register_type::<FocusedWindow>();
        app.register_type::<CursorOnOutput>();
        app.register_type::<CursorOnWindow>();
    }
}

#[derive(Resource)]
pub struct FocusStack(pub LruCache<Entity, ()>);
impl Default for FocusStack {
    fn default() -> Self {
        Self(LruCache::new(NonZeroUsize::new(65535).unwrap()))
    }
}

#[derive(Resource, Default, Reflect, Debug)]
pub struct FocusedWindow(pub Option<Entity>);

#[derive(Resource, Default, Reflect)]
pub struct CursorOnOutput(pub Option<(Entity, IVec2)>);

#[derive(Resource, Default, Reflect)]
pub struct CursorOnWindow(pub Option<(Entity, IVec2)>);

pub fn update_window_stack_by_focus(
    window_focus: Res<FocusedWindow>,
    mut window_stack: ResMut<FocusStack>,
) {
    if window_focus.is_changed() {
        if let Some(focused_window) = window_focus.0.as_ref() {
            window_stack.0.push(*focused_window, ());
        }
    }
}
