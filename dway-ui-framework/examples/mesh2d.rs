use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use dway_ui_derive::color;
use dway_ui_framework::render::mesh::{
    UiMeshBundle, UiMeshHandle, UiMeshMaterialPlugin, UiMeshPlugin,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            UiMeshPlugin,
            UiMeshMaterialPlugin::<ColorMaterial>::default(),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
        ))
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

fn setup(
    mut commands: Commands,
    mut mesh2d_materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((Camera2d::default(), Msaa::Sample4));

    commands
        .spawn(NodeBundle {
            node: Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                width: Val::Px(300.0),
                height: Val::Px(300.0),
                left: Val::Px(64.0),
                top: Val::Px(64.0),
                ..default()
            },
            background_color: (Color::rgb(0.8, 0.8, 0.8)).into(),
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(UiMeshBundle {
                mesh: UiMeshHandle::from(meshes.add(RegularPolygon::new(128.0, 6))),
                material: mesh2d_materials.add(color!("#0000ff")).into(),
                node: Node {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            });
        });
}
