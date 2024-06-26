use std::{f32::consts::PI, time::Duration};

use anyhow::anyhow;
use bevy::{
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
    sprite::Mesh2dHandle,
    winit::WinitPlugin,
};
use dway_tty::{drm::surface::DrmSurface, DWayTTYPlugin};

use dway_util::logger::DWayLogPlugin;
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
                    backends: Some(Backends::VULKAN | Backends::GL),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .set(LogPlugin {
                level: Level::TRACE,
                filter: "dway=debug,dway_server::wl::surface=debug,bevy_ecs=info,naga=info,naga::front=info,bevy_render=debug,bevy_ui=trace,dway_server::input::pointer=info,kayak_ui=info,naga=info,dway-tty=trace".to_string(),
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
    for i in 0..1024 {
        info!("frame {i}");
        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 144.0));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    surface_query: Query<&DrmSurface>,
    mut animations: ResMut<Assets<AnimationClip>>,
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
        commands.spawn((Camera3dBundle {
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

    // The animation API uses the `Name` component to target entities
    let planet = Name::new("planet");
    let orbit_controller = Name::new("orbit_controller");
    let satellite = Name::new("satellite");

    // Creating the animation
    let mut animation = AnimationClip::default();
    // A curve can modify a single part of a transform, here the translation
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Translation(vec![
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, 1.0),
                Vec3::new(-1.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, -1.0),
                // in case seamless looping is wanted, the last keyframe should
                // be the same as the first one
                Vec3::new(1.0, 0.0, 1.0),
            ]),
            interpolation: Interpolation::Linear,
        },
    );
    // Or it can modify the rotation of the transform.
    // To find the entity to modify, the hierarchy will be traversed looking for
    // an entity with the right name at each level
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]),
            interpolation: Interpolation::Linear,
        },
    );
    // If a curve in an animation is shorter than the other, it will not repeat
    // until all other curves are finished. In that case, another animation should
    // be created for each part that would have a different duration / period
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone(), satellite.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0],
            keyframes: Keyframes::Scale(vec![
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
                Vec3::splat(1.2),
                Vec3::splat(0.8),
            ]),
            interpolation: Interpolation::Linear,
        },
    );
    // There can be more than one curve targeting the same entity path
    animation.add_curve_to_path(
        EntityPath {
            parts: vec![planet.clone(), orbit_controller.clone(), satellite.clone()],
        },
        VariableCurve {
            keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
            keyframes: Keyframes::Rotation(vec![
                Quat::IDENTITY,
                Quat::from_axis_angle(Vec3::Y, PI / 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
                Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
                Quat::IDENTITY,
            ]),
            interpolation: Interpolation::Linear,
        },
    );

    // Create the animation player, and set it to repeat
    let mut player = AnimationPlayer::default();
    player.play(animations.add(animation)).repeat();

    // Create the scene that will be animated
    // First entity is the planet
    commands
        .spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(Sphere::default())),
                material: standard_materials.add(Color::rgb(0.8, 0.7, 0.6)),
                ..default()
            },
            // Add the Name component, and the animation player
            planet,
            player,
        ))
        .with_children(|p| {
            // This entity is just used for animation, but doesn't display anything
            p.spawn((
                SpatialBundle::INHERITED_IDENTITY,
                // Add the Name component
                orbit_controller,
            ))
            .with_children(|p| {
                // The satellite, placed at a distance of the planet
                p.spawn((
                    PbrBundle {
                        transform: Transform::from_xyz(1.5, 0.0, 0.0),
                        mesh: meshes.add(Mesh::from(Cuboid {
                            half_size: Vec3::splat(0.5),
                        })),
                        material: standard_materials.add(Color::rgb(0.3, 0.9, 0.3)),
                        ..default()
                    },
                    // Add the Name component
                    satellite,
                ));
            });
        });
}

pub fn animate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<Mesh2dHandle>>) {
    for mut transform in &mut query {
        transform.rotate_local_z(time.delta_seconds());
    }
}

pub fn input_event_system(
    mut move_events: EventReader<MouseMotion>,
    mut whell_event: EventReader<MouseWheel>,
    mut button_event: EventReader<MouseButtonInput>,
    mut keyboard_event: EventReader<KeyboardInput>,
    mut query: Query<&mut Transform, With<Mesh2dHandle>>,
    mut exit: EventWriter<AppExit>,
) {
    for event in move_events.read() {
        dbg!(event);
        for mut transform in &mut query {
            transform.translation += Vec3::new(event.delta.x, event.delta.y, 0.0);
        }
    }
    for event in whell_event.read() {
        dbg!(event);
        for mut transform in &mut query {
            transform.scale += Vec3::new(event.x * 0.01, event.y * 0.01, 0.0);
        }
    }
    for event in button_event.read() {
        dbg!(event);
    }
    for event in keyboard_event.read() {
        if event.key_code == KeyCode::Escape {
            exit.send(AppExit);
        }
        dbg!(event);
    }
}
