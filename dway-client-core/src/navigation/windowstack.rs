use crate::{desktop::FocusedWindow, prelude::*, DWayClientSystem};
use dway_server::xdg::DWayWindow;
use std::collections::LinkedList;

#[derive(Component, Reflect, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowIndex {
    pub global: usize,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WindowIndexDefaultPosition {
    #[default]
    Top,
    Bottom,
    Index(usize),
}
#[derive(Resource, Default)]
pub struct WindowStackConfig {
    pub default_position: WindowIndexDefaultPosition,
}

#[derive(Resource, Default)]
pub struct WindowStack {
    pub list: LinkedList<Entity>,
}
impl WindowStack {
    pub fn remove_entity(&mut self, e: Entity) {
        let mut c = self.list.cursor_front_mut();
        while let Some(n) = c.current() {
            if *n == e {
                c.remove_current();
                break;
            }
            c.move_next();
        }
    }

    pub fn insert_entity(&mut self, i: usize, e: Entity) {
        let mut c = self.list.cursor_front_mut();
        for _ in 0..i {
            c.move_next();
        }
        c.insert_before(e);
    }

    pub fn focused(&self) -> Option<Entity> {
        self.list.front().cloned()
    }
}

#[derive(Event, Clone, Copy)]
pub enum SetWindowIndex {
    ToTop(Entity),
    ToBottom(Entity),
    Insert(Entity, usize),
    Swap(Entity, Entity),
}

pub fn init_window_index(
    config: Res<WindowStackConfig>,
    mut stack: ResMut<WindowStack>,
    new_window_query: Query<Entity, Added<DWayWindow>>,
) {
    new_window_query.into_iter().for_each(|new_window| {
        match config.default_position {
            WindowIndexDefaultPosition::Top => {
                stack.list.push_front(new_window);
            }
            WindowIndexDefaultPosition::Bottom => {
                stack.list.push_back(new_window);
            }
            WindowIndexDefaultPosition::Index(i) => {
                stack.insert_entity(i, new_window);
            }
        };
        debug!(winodw=?new_window, "add window to stack");
    });
}

pub fn update_window_index(
    mut events: EventReader<SetWindowIndex>,
    mut stack: ResMut<WindowStack>,
    mut window_query: Query<Option<&mut WindowIndex>, With<DWayWindow>>,
    mut removed_window_query: RemovedComponents<DWayWindow>,
    mut commands: Commands,
) {
    removed_window_query.read().for_each(|e| {
        stack.remove_entity(e);
    });

    for event in events.read() {
        match event {
            SetWindowIndex::ToTop(e) => {
                if stack.list.front() != Some(&e) {
                    stack.remove_entity(*e);
                    stack.list.push_front(*e);
                }
            }
            SetWindowIndex::ToBottom(e) => {
                if stack.list.back() != Some(&e) {
                    stack.remove_entity(*e);
                    stack.list.push_back(*e);
                }
            }
            SetWindowIndex::Insert(e, i) => {
                stack.remove_entity(*e);
                stack.insert_entity(*i, *e);
            }
            SetWindowIndex::Swap(e0, e1) => {
                let mut c = stack.list.cursor_front_mut();
                while let Some(n) = c.current() {
                    if *n == *e0 {
                        *n = *e1;
                    } else if *n == *e1 {
                        *n = *e0;
                    }
                    c.move_next();
                }
            }
        }
    }

    if stack.is_changed() {
        debug!(
            "window stack: {:?}",
            Vec::from_iter(stack.list.iter().enumerate())
        );
        for (i, e) in stack.list.iter().enumerate() {
            if let Ok(index) = window_query.get_mut(*e) {
                if let Some(mut index) = index {
                    index.global = i;
                } else {
                    commands.entity(*e).insert(WindowIndex { global: i });
                }
            }
        }
    }
}

pub fn update_window_stack_by_focus(
    window_focus: Res<FocusedWindow>,
    mut window_stack: ResMut<WindowStack>,
) {
    if window_focus.is_changed() {
        if let Some(focused_window) = window_focus.window_entity.as_ref() {
            if window_stack.list.front() != Some(&focused_window) {
                debug!(window=?*focused_window, "move focused window to top of stack");
                window_stack.remove_entity(*focused_window);
                window_stack.list.push_front(*focused_window);
            }
        }
    }
}

pub struct WindowStackPlugin;
impl Plugin for WindowStackPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WindowIndex>()
            .init_resource::<WindowStackConfig>()
            .init_resource::<WindowStack>()
            .add_event::<SetWindowIndex>()
            .add_systems(
                PreUpdate,
                (
                    init_window_index.in_set(DWayClientSystem::CreateComponent),
                    update_window_index.in_set(DWayClientSystem::UpdateZIndex),
                    update_window_stack_by_focus
                        .run_if(resource_changed::<FocusedWindow>())
                        .in_set(DWayClientSystem::UpdateFocus),
                ),
            );
    }
}
