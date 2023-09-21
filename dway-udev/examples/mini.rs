use std::fs::OpenOptions;

use bevy::{
    app::ScheduleRunnerPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    log::LogPlugin,
    prelude::*,
    render::{camera::RenderTarget, RenderPlugin},
    winit::WinitPlugin,
};
use dway_udev::DWayTTYPlugin;
use tracing::Level;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

const THREAD_POOL_CONFIG: TaskPoolThreadAssignmentPolicy = TaskPoolThreadAssignmentPolicy {
    min_threads: 1,
    max_threads: 1,
    percent: 0.25,
};

pub fn main() {
    let mut app = App::new();
    app.add_plugins(
            DefaultPlugins
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
                }})
                .set(LogPlugin {
                    level: Level::INFO,
                    filter: "info,dway=debug,dway_server::wl::surface=debug,bevy_ecs=info,wgpu=info,wgpu_hal::gles=info,naga=info,naga::front=info,bevy_render=debug,bevy_ui=trace,dway_server::input::pointer=info,kayak_ui=info,naga=info,dway-udev=trace".to_string(),
                })
                .disable::<WinitPlugin>(),
        )
        .insert_resource(ClearColor(Color::rgb(1.0, 1.0, 1.0)))
        .add_plugin(DWayTTYPlugin::default())
        .add_system(setup.on_startup())
        .add_system(animate_cube);
    app.setup();
    for i in 0..4 {
        info!("frame {i}");
        app.update();
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("camera render target"),
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    image.resize(size);
    let image_handle = images.add(image);

    // commands.spawn((PbrBundle {
    //     mesh: meshes.add(shape::Cube::default().into()),
    //     material: materials.add(Color::ORANGE.into()),
    //     transform: Transform::default(),
    //     ..default()
    // },));
    // commands.spawn(PointLightBundle {
    //     point_light: PointLight {
    //         intensity: 9000.,
    //         range: 100.,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     transform: Transform::from_xyz(8., 16., 8.),
    //     ..default()
    // });
    // commands.spawn(PbrBundle {
    //     mesh: meshes.add(shape::Plane::from_size(50.).into()),
    //     material: materials.add(Color::SILVER.into()),
    //     ..default()
    // });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 6., 12.).looking_at(Vec3::new(0., 3., 0.), Vec3::Y),
        camera: Camera {
            // render before the "main pass" camera
            order: -1,
            target: RenderTarget::Image(image_handle.clone()),
            ..default()
        },
        ..default()
    });
}

pub fn animate_cube(time: Res<Time>, mut query: Query<&mut Transform>) {
    for mut transform in &mut query {
        transform.rotate_x(1.0);
    }
}
