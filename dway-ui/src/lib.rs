#![feature(arc_unwrap_or_clone)]
pub mod framework;
pub mod panels;
pub mod prelude;
pub mod util;
pub mod widgets;

use crate::prelude::*;
use bevy::{render::camera::RenderTarget, ui::FocusPolicy};
use dway_tty::{drm::surface::DrmSurface, seat::SeatState};
use font_kit::{
    error::SelectionError, family_name::FamilyName, properties::Properties, source::SystemSource,
};
use widgets::applist::{AppListUI, AppListUIBundle};

pub struct DWayUiPlugin;
impl Plugin for DWayUiPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins((framework::UiFrameworkPlugin, widgets::DWayWidgetsPlugin));
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
    Some(loaded.full_name())
}

fn setup(
    mut commands: Commands,
    seat: Option<NonSend<SeatState>>,
    surfaces: Query<&DrmSurface>,
    asset_server: Res<AssetServer>,
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

    commands.spawn(ImageBundle {
        style: styled!("absolute w-full h-full"),
        image: asset_server.load("background.jpg").into(),
        z_index: ZIndex::Global(-1024),
        ..default()
    });
    commands
        .spawn((
            Name::new("applist-ui"),
            NodeBundle {
                style: styled!("absolute bottom-4 w-full justify-center items-center"),
                focus_policy: FocusPolicy::Pass,
                z_index: ZIndex::Global(1024),
                ..default()
            },
        ))
        .with_children(|c| {
            c.spawn(widgets::applist::AppListPanelBundle::default());
        });
}
