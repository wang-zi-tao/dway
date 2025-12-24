use std::time::Duration;

use bevy::{
    app::AppExit,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::tonemapping::Tonemapping,
    input::keyboard::KeyboardInput,
    log::LogPlugin,
    prelude::*,
    render::{
        camera::RenderTarget,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    winit::WinitPlugin,
};
use dway_tty::{drm::surface::DrmSurface, DWayTTYPlugin};
use tracing::Level;
use wgpu::Backends;

pub fn main() {
    let mut app = App::new();
    app.add_plugins({
        let mut plugins = DefaultPlugins
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    backends: Some(Backends::GL),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(LogPlugin {
                level: Level::DEBUG,
                filter: "dway=debug,dway_server::wl::surface=debug,bevy_ecs=info,naga=info,naga::front=info,bevy_render=trace,bevy_ui=trace,dway_server::input::pointer=info,kayak_ui=info,naga=info,dway-tty=trace".to_string(),
                ..Default::default()
            });
            if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
                plugins = plugins
                    .disable::<WinitPlugin>()
                    .add(DWayTTYPlugin::default())
            }
            plugins
        })
        .insert_resource(ClearColor(Color::WHITE))
        .add_systems(Startup,setup)
        .add_systems(Update,animate_cube)
        .add_systems(Update,input_event_system);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    surface_query: Query<&DrmSurface>,
) {
    info!("setup world");

    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        commands.spawn((
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera3d::default(),
            IsDefaultUiCamera,
        ));
        info!("setup camera");
    }

    for (i, surface) in surface_query.iter().enumerate() {
        let image_handle = surface.image();
        let mut entity_command = commands.spawn((
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
        ));
        if i == 0 {
            entity_command.insert(IsDefaultUiCamera);
        }
        info!("setup camera");
    }

    commands.spawn((
        Text::new("drm3d"),
        TextFont {
            font_size: 24.0,
            ..Default::default()
        },
        TextColor(Color::rgba(1.0, 0.0, 0.0, 1.0)),
        Node {
            left: Val::Px(64.0),
            top: Val::Px(64.0),
            ..Default::default()
        },
    ));

    commands.spawn((
        BackgroundColor(Color::rgb(0.8, 0.8, 0.8)),
        Node {
            width: Val::Px(64.),
            height: Val::Px(64.),
            ..Default::default()
        },
    ));

    commands.spawn((
        PointLight {
            intensity: 10_000_000.,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(2.0, 2.5, 2.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Cuboid::default()))),
        MeshMaterial3d(
            standard_materials.add(StandardMaterial::from_color(Color::linear_rgb(
                0.0, 0.0, 1.0,
            ))),
        ),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Plane3d::new(Vec3::Y, Vec2::splat(16.0))))),
        MeshMaterial3d(
            standard_materials.add(StandardMaterial::from_color(Color::rgb(0.5, 0.5, 0.5))),
        ),
        Transform::from_xyz(0.0, -1.0, 0.0),
    ));
}

pub fn animate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<Mesh3d>>) {
    for mut transform in &mut query {
        transform.rotate_local_y(time.delta_secs());
    }
}

pub fn input_event_system(
    mut keyboard_event: MessageReader<KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
) {
    for event in keyboard_event.read() {
        if event.key_code == KeyCode::Escape {
            exit.write(AppExit::Success);
        }
        info!("receive keyboard input: {:?}", event);
    }
}
