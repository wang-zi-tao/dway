use std::{f32::consts::PI, thread, time::Duration};

use bevy::{
    app::ScheduleRunnerPlugin,
    audio::AudioPlugin,
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::CorePipelinePlugin,
    diagnostic::{DiagnosticsPlugin, FrameTimeDiagnosticsPlugin},
    gltf::GltfPlugin,
    input::InputPlugin,
    log::{Level, LogPlugin},
    pbr::PbrPlugin,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        settings::{Backends, WgpuSettings},
        RenderApp, RenderPlugin, RenderStage,
    },
    scene::ScenePlugin,
    sprite::SpritePlugin,
    text::TextPlugin,
    time::TimePlugin,
    ui::UiPlugin,
    window::PresentMode,
    winit::{UpdateMode, WinitPlugin, WinitSettings},
};
use dway_udev::UDevBackendPlugin;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

fn main() {
    let mut app = App::new();
    // .insert_resource(ClearColor(Color::rgb(0.0, 0.388, 1.0)))
    app.insert_resource(Time::default());
    // app.insert_resource(WinitSettings::game());
    // app.insert_resource(WinitSettings {
    //     focused_mode: UpdateMode::ReactiveLowPower {
    //         max_wait: Duration::from_secs_f64(1.0 / 70.0),
    //     },
    //     unfocused_mode: UpdateMode::ReactiveLowPower {
    //         max_wait: Duration::from_secs_f64(1.0 / 70.0),
    //     },
    //     return_from_run: true,
    //     // unfocused_mode: UpdateMode::ReactiveLowPower {
    //     //     max_wait: Duration::from_secs(60),
    //     // },
    //     ..Default::default()
    // });
    // app.add_plugins(MinimalPlugins);

    app.add_plugin(CorePlugin {
        task_pool_options: TaskPoolOptions {
            compute: TaskPoolThreadAssignmentPolicy {
                min_threads: 0,
                max_threads: 0,
                percent: 2.0,
            },
            ..CorePlugin::default().task_pool_options
        },
    })
    .add_plugin(LogPlugin {
        level: Level::INFO,
        filter: "dway=debug,wgpu_core=warn,wgpu_hal=warn".to_string(),
    })
    .add_plugin(TimePlugin::default())
    // .add_plugin(ScheduleRunnerPlugin::default())
    .add_plugin(TransformPlugin::default())
    .add_plugin(HierarchyPlugin::default())
    .add_plugin(DiagnosticsPlugin::default())
    .add_plugin(InputPlugin::default())
    .add_plugin(WindowPlugin {
        window: WindowDescriptor {
            title: "dway".to_string(),
            present_mode: PresentMode::AutoVsync,
            ..default()
        },
        ..default()
    })
    .add_plugin(AssetPlugin::default())
    .add_plugin(ScenePlugin::default())
    // .add_plugin(WinitPlugin::default())
    // .insert_resource(WgpuSettings{
    //         // backends:None,
    //         ..Default::default()})
    .add_plugin(UDevBackendPlugin::default())
    .insert_resource(WgpuSettings {
        backends: Some(Backends::GL),
        ..Default::default()
    })
    .add_plugin(RenderPlugin::default())
    .add_plugin(ImagePlugin::default())
    .add_plugin(CorePipelinePlugin::default())
    .add_plugin(SpritePlugin::default())
    .add_plugin(TextPlugin::default())
    .add_plugin(UiPlugin::default())
    .add_plugin(PbrPlugin::default())
    .add_plugin(GltfPlugin::default())
    .add_plugin(AudioPlugin::default())
    .add_plugin(GilrsPlugin::default())
    .add_plugin(AnimationPlugin::default());

    app.add_plugin(bevy::diagnostic::LogDiagnosticsPlugin::default());
    app.add_plugin(bevy::diagnostic::EntityCountDiagnosticsPlugin::default());
    // .add_plugin(AssetCountDiagnosticsPlugin::<Image>::default())

    app.add_plugin(FrameTimeDiagnosticsPlugin::default());

    // app.add_plugin(WorldInspectorPlugin::new());
    // .add_startup_system(hello_world)

    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_system_to_stage(
        RenderStage::Extract,
        debug.before(bevy::render::view::WindowSystem::Prepare),
    );

    app.add_startup_system(setup);

    app.run();
}

pub struct RawHandle(RawWindowHandle, RawDisplayHandle);
unsafe impl HasRawWindowHandle for RawHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}
unsafe impl HasRawDisplayHandle for RawHandle {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.1
    }
}

fn debug(
    windows: Res<bevy::render::view::ExtractedWindows>,
    instance: Res<bevy::render::renderer::RenderInstance>,
    render_adapter: Res<bevy::render::renderer::RenderAdapter>,
) {
    for (id, window) in windows.iter() {
        unsafe {
            dbg!("create_surface");
            let surface = instance
                .0
                .create_surface(&window.raw_handle.as_ref().unwrap().get_handle());
            dbg!(&surface);
            dbg!(render_adapter.0.features());
            let format = surface.get_supported_formats(&render_adapter);
            dbg!(&format);
            panic!();
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animations: ResMut<Assets<AnimationClip>>,
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
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            // render before the "main pass" camera
            priority: -1,
            target: RenderTarget::Image(image_handle.clone()),
            ..default()
        },
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
        },
    );
    // Or it can modify the rotation of the transform.
    // To find the entity to modify, the hierarchy  will be traversed looking for
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
                mesh: meshes.add(Mesh::try_from(shape::Icosphere::default()).unwrap()),
                material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                ..default()
            },
            // Add the Name component, and the animation player
            planet,
            player,
        ))
        .with_children(|p| {
            // This entity is just used for animation, but doesn't display anything
            p.spawn((
                SpatialBundle::VISIBLE_IDENTITY,
                // Add the Name component
                orbit_controller,
            ))
            .with_children(|p| {
                // The satellite, placed at a distance of the planet
                p.spawn((
                    PbrBundle {
                        transform: Transform::from_xyz(1.5, 0.0, 0.0),
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                        material: materials.add(Color::rgb(0.3, 0.9, 0.3).into()),
                        ..default()
                    },
                    // Add the Name component
                    satellite,
                ));
            });
        });
    info!("setup");
}
