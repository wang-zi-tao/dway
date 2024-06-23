use std::num::NonZeroUsize;

use bevy::prelude::*;
use dway_server::apps::AppRef;
use lru::LruCache;

use crate::DWayClientSystem;

pub struct DWayDesktop;
impl Plugin for DWayDesktop {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_resource(FocusStack::default());
        app.insert_resource(FocusedWindow::default());
        app.insert_resource(CursorOnOutput::default());
        app.insert_resource(CursorOnWindow::default());
        app.register_type::<FocusedWindow>();
        app.register_type::<CursorOnOutput>();
        app.register_type::<CursorOnWindow>();
        app.add_systems(
            PreUpdate,
            update_window_stack_by_focus
                .run_if(resource_changed::<FocusedWindow>)
                .in_set(DWayClientSystem::UpdateWindowStack),
        );
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
pub struct FocusedWindow {
    pub window_entity: Option<Entity>,
    pub app_entity: Option<Entity>,
}

#[derive(Resource, Default, Reflect, Debug)]
pub struct CursorOnOutput(pub Option<(Entity, IVec2)>);

impl CursorOnOutput {
    pub fn get_screen(&self) -> Option<Entity> {
        self.0.as_ref().map(|x| x.0)
    }
}

#[derive(Resource, Default, Reflect, Debug)]
pub struct CursorOnWindow(pub Option<(Entity, IVec2)>);
impl CursorOnWindow {
    pub fn get_window(&self) -> Option<Entity> {
        self.0.as_ref().map(|x| x.0)
    }
}

pub fn update_window_stack_by_focus(
    window_query: Query<&AppRef>,
    mut window_focus: ResMut<FocusedWindow>,
    mut window_stack: ResMut<FocusStack>,
) {
    if window_focus.is_changed() {
        if let Some(focused_window) = window_focus.window_entity.as_ref() {
            window_stack.0.push(*focused_window, ());
            window_focus.app_entity = window_query
                .get(*focused_window)
                .ok()
                .and_then(|app_ref| app_ref.get());
        }
    }
}
