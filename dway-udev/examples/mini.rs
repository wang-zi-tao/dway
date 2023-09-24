use std::{fs::OpenOptions, time::Duration};

use bevy::{
    app::ScheduleRunnerPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::clear_color::ClearColorConfig,
    input::{mouse::{MouseMotion, MouseWheel, MouseButtonInput}, keyboard::KeyboardInput},
    log::LogPlugin,
    prelude::*,
    render::{camera::RenderTarget, RenderPlugin},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    winit::WinitPlugin,
};
use dway_udev::{drm::surface::DrmSurface, DWayTTYPlugin};
use input::event::pointer::PointerAxisEvent;
use tracing::Level;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

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
                wgpu_settings: bevy::render::settings::WgpuSettings {
                    backends: Some(wgpu::Backends::GL),
                    ..Default::default()
                },
            })
            .set(TaskPoolPlugin {
                task_pool_options: TaskPoolOptions {
                    min_total_threads: 1,
                    max_total_threads: 1,
                    io: THREAD_POOL_CONFIG,
                    async_compute: THREAD_POOL_CONFIG,
                    compute: THREAD_POOL_CONFIG,
                },
            })
            .set(LogPlugin {
                level: Level::INFO,
                filter: "info,dway=debug,dway_server::wl::surface=debug,bevy_ecs=info,wgpu=info,wgpu_hal::gles=info,naga=info,naga::front=info,bevy_render=debug,bevy_ui=trace,dway_server::input::pointer=info,kayak_ui=info,naga=info,dway-udev=trace".to_string(),
            });
            if std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
                plugins = plugins
                    .disable::<WinitPlugin>()
                    .add(DWayTTYPlugin::default())
            }
            plugins
        })
        .insert_resource(ClearColor(Color::rgb(1.0, 0.5, 1.0)))
        .add_system(setup.on_startup())
        .add_system(animate_cube)
        .add_system(input_event_system);
    app.setup();
    for i in 0..256 {
        info!("frame {i}");
        app.update();
        std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0));
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut surface_query: Query<&DrmSurface>,
) {
    info!("setup world");

    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        commands.spawn(Camera2dBundle::default());
    }
    surface_query.for_each(|surface| {
        let image_handle = surface.image();
        commands.spawn(Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::BLUE),
            },
            camera: Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            ..default()
        });
        // commands.spawn((Camera3dBundle {
        //     transform: Transform::from_xyz(0., 6., 12.).looking_at(Vec3::new(0., 3., 0.), Vec3::Y),
        //     camera: Camera {
        //         target: RenderTarget::Image(image_handle),
        //         ..default()
        //     },
        //     ..default()
        // },));
        info!("setup camera");
    });

    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Cube::new(400.0).into()).into(),
        material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
        ..default()
    });

    commands.spawn((PbrBundle {
        mesh: meshes.add(shape::Cube::default().into()),
        material: standard_materials.add(Color::ORANGE.into()),
        transform: Transform::default(),
        ..default()
    },));
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.,
            range: 100.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8., 16., 8.),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.).into()),
        material: standard_materials.add(Color::SILVER.into()),
        ..default()
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
) {
    for event in move_events.iter() {
        dbg!(event);
        for mut transform in &mut query {
            transform.translation += Vec3::new(event.delta.x, event.delta.y, 0.0);
        }
    }
    for event in whell_event.iter() {
        dbg!(event);
        for mut transform in &mut query {
            transform.scale += Vec3::new(event.x * 0.01, event.y * 0.01, 0.0);
        }
    }
    for event in button_event.iter() {
        dbg!(event);
    }
    for event in keyboard_event.iter() {
        dbg!(event);
    }
}
