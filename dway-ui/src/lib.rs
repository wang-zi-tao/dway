#![feature(arc_unwrap_or_clone)]
pub mod prelude;
pub mod util;
pub mod widgets;

use bevy::{prelude::*, render::camera::RenderTarget};
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(widgets::DWayWidgetsPlugin);
        app.add_systems(Startup, setup);
    }
}

fn setup(
    mut commands: Commands,
    seat: Option<NonSend<SeatState>>,
    surfaces: Query<&DrmSurface>,
) {
    if seat.is_none() {
        let camera = Camera2dBundle::default();
        commands.spawn(camera);
    } else {
        surfaces.for_each(|surface| {
            let image_handle = surface.image();
            commands.spawn((Camera2dBundle {
                camera: Camera {
                    target: RenderTarget::Image(image_handle),

                    ..default()
                },
                ..default()
            },));
        });
    }
}
