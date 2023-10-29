#![feature(arc_unwrap_or_clone)]
pub mod prelude;
pub mod util;
pub mod widgets;

use bevy::{prelude::*, render::camera::RenderTarget};
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};
use font_kit::{
    error::SelectionError, family_name::FamilyName, properties::Properties, source::SystemSource,
};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(widgets::DWayWidgetsPlugin);
        app.add_systems(Startup, setup);
    }
}

pub fn default_system_font() -> Option<String> {
    let source = SystemSource::new();
    let default_fonts = &[
        FamilyName::Title("arial".to_string()),
        FamilyName::SansSerif,
        FamilyName::Monospace,
        FamilyName::Fantasy,
    ];
    let font = source
        .select_best_match(
            default_fonts,
            Properties::new().style(font_kit::properties::Style::Normal),
        )
        .ok()?;
    let loaded = font.load().ok()?;
    dbg!(&loaded);
    dbg!(&loaded.full_name());
    Some(loaded.full_name())
}

fn setup(mut commands: Commands, seat: Option<NonSend<SeatState>>, surfaces: Query<&DrmSurface>) {
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
