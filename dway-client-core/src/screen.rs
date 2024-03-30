use crate::prelude::*;
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
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

pub struct ScreenPlugin;
impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            create_screen.in_set(DWayClientSystem::CreateComponent),
        );
    }
}
