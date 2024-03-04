use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    reflect::TypePath,
    sprite::Mesh2dHandle,
};
use bevy_prototype_lyon::{
    draw::{Fill, Stroke},
    entity::{Path, ShapeBundle},
    geometry::GeometryBuilder,
    path::PathBuilder,
    shapes,
};
use chrono::Timelike;
use dway_ui_framework::{
    render::mesh::{UiMeshBundle, UiMeshHandle, UiMeshMaterialPlugin, UiMeshPlugin},
    widgets::shape::UiShapeBundle,
    UiFrameworkPlugin,
};
use std::{
    f32::consts::PI,
    time::{Duration, SystemTime},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            UiFrameworkPlugin,
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

#[derive(Component)]
pub struct Clock;

fn setup(
    mut commands: Commands,
    mut mesh2d_materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        Clock,
        Stroke::new(Color::BLACK, 10.0),
        UiShapeBundle {
            style: Style {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                width: Val::Px(256.0),
                height: Val::Px(256.0),
                ..default()
            },
            ..default()
        },
    ));
}
fn update(mut query: Query<&mut Path, With<Clock>>, time: Res<Time>) {
    for mut path in &mut query {
        let time = chrono::offset::Local::now();
        let m = time.minute() as f32;
        let s = time.second() as f32 + time.timestamp_subsec_millis() as f32 / 1000.0;

        let polar2rectangular = |angle: f32, l: f32| Vec2::new(angle.sin(), angle.cos()) * l;

        let mut builder = PathBuilder::new();

        builder.move_to(polar2rectangular(m * 2. * PI / 60.0, 96.0));
        builder.line_to(Vec2::ZERO);
        builder.line_to(polar2rectangular(s * 2. * PI / 60.0, 128.0));

        *path = builder.build();
    }
}
