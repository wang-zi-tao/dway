use std::collections::HashSet;

use bevy::prelude::*;
use bevy_relationship::{graph_query2, ControlFlow};
use dway_server::{
    events::Insert, geometry::GlobalGeometry, macros::{WindowAction}, xdg::{
        DWayWindow, toplevel::{DWayToplevel, PinedWindow}
    }
};
use dway_util::update;
use getset::Getters;

use crate::{
    layout::LayoutStyle,
    screen::{ScreenContainsWindow, WindowScreenList},
    DWayClientSystem,
};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Hidden;

#[derive(Component, Reflect, Clone, Debug, Default)]
pub struct WindowClientInfo {
    pub max: bool,
    pub min: bool,
    pub fullscreen: bool,
}

#[derive(Component, Getters, Default)]
pub struct WindowStatistics {
    #[get(copy)]
    fullscreen: bool,
    #[get(copy)]
    max: bool,
}

pub fn on_window_created(mut new_windows: MessageReader<Insert<DWayWindow>>, mut commands: Commands) {
    for new_window in new_windows.read() {
        let entity = new_window.entity;
        commands.queue(move |world: &mut World| {
            if let Ok(mut e) = world.get_entity_mut(entity) {
                e.insert(WindowClientInfo::default());
            }
        })
    }
}

pub fn update_window(
    mut window_query: Query<
        (
            Entity,
            Ref<DWayToplevel>,
            &mut WindowClientInfo,
            &WindowScreenList,
        ),
        Changed<DWayToplevel>,
    >,
    screen_query: Query<(&GlobalGeometry, Option<&LayoutStyle>)>,
    mut commands: Commands,
    mut window_actions: MessageWriter<WindowAction>,
) {
    for (window_entity, window, mut client, screen_list) in &mut window_query {
        if window.is_changed() {
            update!(client.max, window.max, {
                if window.max {
                    if let Some((screen_geo, layout_style)) =
                        screen_query.iter_many(screen_list.iter()).next()
                    {
                        let rect = layout_style
                            .map(|s| s.get_pedding_rect(screen_geo.geometry))
                            .unwrap_or(screen_geo.geometry);
                        window_actions.write(WindowAction::SetRect(window_entity, rect));
                        commands.entity(window_entity).insert(PinedWindow);
                    }
                    commands.entity(window_entity).insert(PinedWindow);
                } else {
                    commands.entity(window_entity).remove::<PinedWindow>();
                }
            });
            update!(client.fullscreen, window.fullscreen, {
                if window.fullscreen {
                    if let Some((screen_geo, layout_style)) =
                        screen_query.iter_many(screen_list.iter()).next()
                    {
                        let rect = layout_style
                            .map(|s| s.get_pedding_rect(screen_geo.geometry))
                            .unwrap_or(screen_geo.geometry);
                        window_actions.write(WindowAction::SetRect(window_entity, rect));
                        commands.entity(window_entity).insert(PinedWindow);
                    }
                    commands.entity(window_entity).insert(PinedWindow);
                } else {
                    commands.entity(window_entity).remove::<PinedWindow>();
                }
            });
            update!(client.min, window.min);
        }
    }
}

graph_query2! {
WindowSatisticsGraph=>
   mut windows=match
    (screen: (Entity, &mut WindowStatistics) where ?)-[ScreenContainsWindow]->(window: Ref<DWayToplevel>)
}

pub fn window_statistics_system(mut graph: WindowSatisticsGraph) {
    let mut changed = HashSet::new();
    graph.foreach_windows(
        |_| true,
        |(screen_entity, _), window| {
            if window.is_changed() {
                changed.insert(*screen_entity);
                ControlFlow::Break
            } else {
                ControlFlow::continue_iter()
            }
        },
    );

    for screen_entity in changed {
        graph.foreach_windows_mut_from(
            screen_entity,
            |(_, window)| {
                window.fullscreen = false;
                window.max = false;
                true
            },
            |(_, stat), toplevel| {
                stat.max |= toplevel.max;
                stat.fullscreen |= toplevel.fullscreen;
                ControlFlow::continue_iter()
            },
        );
    }
}

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WindowClientInfo>().add_systems(
            PreUpdate,
            (
                on_window_created
                    .run_if(on_event::<Insert<DWayWindow>>)
                    .in_set(DWayClientSystem::InsertWindowComponent),
                window_statistics_system.in_set(DWayClientSystem::UpdateScreen),
                update_window.in_set(DWayClientSystem::UpdateWindow),
            ),
        );
    }
}
