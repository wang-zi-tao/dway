use bevy::{
    app::AppExit,
    core_pipeline::tonemapping::Tonemapping,
    input::keyboard::KeyboardInput,
    log::LogPlugin,
    math::FloatOrd,
    prelude::*,
    render::{
        camera::{ImageRenderTarget, RenderTarget},
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
        .add_systems(Update,input_event_system);
    app.run();
}

fn setup(mut commands: Commands, surface_query: Query<&DrmSurface>) {
    info!("setup world");

    if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
        commands.spawn((
            Camera2d::default(),
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Tonemapping::None,
        ));
        info!("setup camera");
    }
    for surface in surface_query.iter() {
        commands.spawn((
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera2d::default(),
            Camera {
                target: RenderTarget::Image(ImageRenderTarget {
                    handle: surface.image(),
                    scale_factor: FloatOrd(1.0),
                }),
                ..default()
            },
            Tonemapping::None,
        ));
        info!("setup camera");
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
