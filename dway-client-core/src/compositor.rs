use std::time::Duration;

use bevy::{
    app::ScheduleRunnerSettings,
    asset::diagnostic::AssetCountDiagnosticsPlugin,
    diagnostic::{
        Diagnostics, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    },
    log::{Level, LogPlugin},
    prelude::*,
    window::PresentMode,
    winit::WinitSettings,
};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, DebugEventsPickingPlugin, DefaultPickingPlugins, PickingCameraBundle,
};

use crate::stages::DWayStage;

use super::{
    desktop::DWayDesktop,
    input::DWayInputPlugin,
    protocol::{WindowMessageReceiver, WindowMessageSender},
    window::DWayWindowPlugin,
    WaylandPlugin,
};
pub struct CompositorPlugin;
impl Plugin for CompositorPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_system(fps_update_system);
    }
}

#[derive(Component)]
struct FpsText;
pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "FPS: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 30.0,
                    color: Color::WHITE,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                color: Color::GOLD,
            }),
        ])
        .with_style(Style {
            margin:UiRect::top(Val::Px(32.0)),
            ..Default::default()
        })
        .with_text_alignment(TextAlignment::TOP_LEFT),
        FpsText,
    ));
}
fn fps_update_system(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsText>>) {
    for mut text in &mut query {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.sections[1].value = format!("{value:.2}");
            }
        }
    }
}
