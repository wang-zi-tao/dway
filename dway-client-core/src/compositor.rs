use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
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
        TextBundle {
            z_index: ZIndex::Global(1),
            ..TextBundle::from_sections([
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
                margin: UiRect::top(Val::Px(32.0)),
                ..Default::default()
            })
            .with_text_alignment(TextAlignment::Left)
        },
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
