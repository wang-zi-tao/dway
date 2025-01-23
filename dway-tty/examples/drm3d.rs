use std::{f32::consts::PI, time::Duration};

use anyhow::anyhow;
use bevy::{
    animation::{AnimationTarget, AnimationTargetId},
    app::AppExit,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::tonemapping::Tonemapping,
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    log::LogPlugin,
    prelude::*,
    render::{
        camera::RenderTarget,
        settings::{RenderCreation, WgpuSettings},
        RenderPlugin,
    },
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    winit::WinitPlugin,
};
use dway_tty::{drm::surface::DrmSurface, DWayTTYPlugin};
use dway_util::logger::{log_layer, DWayLogPlugin};
use tracing::Level;
use wgpu::Backends;

const THREAD_POOL_CONFIG: TaskPoolThreadAssignmentPolicy = TaskPoolThreadAssignmentPolicy {
    min_threads: 1,
    max_threads: 1,
    percent: 0.25,
};

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
    app.finish();
    app.cleanup();
    for i in 0..64 {
        info!("frame {i}");
        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 144.0));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut materials_2d: ResMut<Assets<ColorMaterial>>,
    surface_query: Query<&DrmSurface>,
) {
    info!("setup world");

    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        commands.spawn((Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            tonemapping: Tonemapping::None,
            ..default()
        },));
        info!("setup camera");
    }
    for surface in surface_query.iter() {
        let image_handle = surface.image();
        commands.spawn((Camera2dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                target: RenderTarget::Image(image_handle),
                ..default()
            },
            tonemapping: Tonemapping::None,
            ..default()
        },));
        info!("setup camera");
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 36000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 1.5, 0.0),
        ..default()
    });

    commands.spawn(NodeBundle{
        background_color: BackgroundColor(Color::rgb(0.8, 0.8, 0.8)),
        style: Style {
            width: Val::Px(64.),
            height: Val::Px(64.),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(Sphere::default())),
        material: standard_materials.add(Color::rgb(0.8, 0.7, 0.6)),
        ..default()
    });
}

pub fn animate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<Mesh2dHandle>>) {
    for mut transform in &mut query {
        transform.rotate_local_z(time.delta_seconds());
    }
}

pub fn input_event_system(
    mut keyboard_event: EventReader<KeyboardInput>,
    mut exit: EventWriter<AppExit>,
) {
    for event in keyboard_event.read() {
        if event.key_code == KeyCode::Escape {
            exit.send(AppExit::Success);
        }
        dbg!(event);
    }
}
