use crate::prelude::*;
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::DWayWindow,
};

#[derive(Component)]
pub struct Screen {
    pub name: String,
}

pub fn create_screen(
    screen_query: Query<(Entity, Ref<Window>, Option<&Screen>), Changed<Window>>,
    mut commands: Commands,
) {
    for (entity, window, screen) in screen_query.iter() {
        let WindowPosition::At(window_position) = window.position else {
            continue;
        };
        let rect = IRect::new(
            window_position.x,
            window_position.y,
            window.resolution.width() as i32,
            window.resolution.height() as i32,
        );
        if screen.is_none() {
            commands.entity(entity).insert((
                Screen {
                    name: window.title.clone(),
                },
                Name::new(window.title.clone()),
                Geometry::new(rect),
                GlobalGeometry::new(rect),
            ));
        }
    }
}

relationship!(ScreenShowWindow=>ScreenWindowList>-<WindowScreenList);

pub fn update_screen(
    screen_query: Query<(Entity, Ref<GlobalGeometry>)>,
    window_query: Query<(Entity, Ref<GlobalGeometry>, Ref<DWayWindow>)>,
    mut commands: Commands,
) {
    for (window_entity, window_geo, window) in &window_query {
        if window_geo.is_changed() || window.is_changed() {
            commands
                .entity(window_entity)
                .disconnect_all_rev::<ScreenShowWindow>();
            for (screen_entity, screen_geo) in &screen_query {
                if !screen_geo.intersection(window_geo.geometry).empty() {
                    commands
                        .entity(screen_entity)
                        .connect_to::<ScreenShowWindow>(window_entity);
                }
            }
        }
    }
    for (screen_entity, screen_geo) in &screen_query {
        if screen_geo.is_changed() {
            commands
                .entity(screen_entity)
                .disconnect_all::<ScreenShowWindow>();
            for (window_entity, window_geo, _) in &window_query {
                if !screen_geo.intersection(window_geo.geometry).empty() {
                    commands
                        .entity(screen_entity)
                        .connect_to::<ScreenShowWindow>(window_entity);
                }
            }
        }
    }
}

pub struct ScreenPlugin;
impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<ScreenShowWindow>();
        app.add_systems(
            PreUpdate,
            (
                create_screen.in_set(DWayClientSystem::CreateComponent),
                update_screen.in_set(DWayClientSystem::CreateComponent),
            )
                .chain(),
        );
    }
}
