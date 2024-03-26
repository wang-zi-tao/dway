use std::time::Duration;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::sprite::Mesh2dHandle;
use dway_ui_framework::render::mesh::UiMeshBundle;
use dway_ui_framework::render::mesh::UiMeshHandle;
use dway_ui_framework::render::mesh::UiMeshMaterialPlugin;
use dway_ui_framework::render::mesh::UiMeshPlugin;

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
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                width: Val::Px(300.0),
                height: Val::Px(300.0),
                left: Val::Px(64.0),
                top: Val::Px(64.0),
                ..default()
            },
            background_color: (Color::WHITE *0.8).into(),
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(UiMeshBundle {
                mesh: UiMeshHandle::from(meshes.add(RegularPolygon::new(128.0, 6))),
                material: mesh2d_materials.add(Color::BLUE),
                style: Style {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                ..default()
            });
        });
    commands.spawn(ColorMesh2dBundle {
        transform: Transform::default().with_translation(Vec3::new(100.0, 200.0, 0.0)),
        mesh: Mesh2dHandle::from(meshes.add(RegularPolygon::new(128.0, 8))),
        material: mesh2d_materials.add(Color::RED),
        ..Default::default()
    });
}
