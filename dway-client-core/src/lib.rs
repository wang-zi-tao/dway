use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_mod_picking::{
    DebugCursorPickingPlugin, DebugEventsPickingPlugin, DefaultPickingPlugins, PickableBundle,
    PickingCameraBundle,
};
use log::info;
use stages::DWayStage;

use self::{desktop::WindowSet, window::receive_window_message};
pub mod compositor;
pub mod desktop;
pub mod input;
pub mod protocol;
pub mod render;
pub mod screen;
pub mod stages;
pub mod window;
pub mod workspace;
pub mod moving;
pub mod resizing;

pub struct WaylandPlugin;

impl Plugin for WaylandPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_state(DWayStage::Desktop);
        app.add_plugin(compositor::CompositorPlugin);
        app.add_plugin(DebugCursorPickingPlugin);
        app.add_plugin(DebugEventsPickingPlugin);
        app.add_plugins(DefaultPickingPlugins);
        app.add_startup_system(setup_2d);
        app.add_plugin(input::DWayInputPlugin { debug: true });
        app.add_plugin(desktop::DWayDesktop);
        app.add_plugin(window::DWayWindowPlugin);
        app.add_plugin(moving::DWayMovingPlugin::default());
        app.add_plugin(resizing::DWayResizingPlugin::default());
        // app.add_system(debug_info);
    }
}

pub fn debug_info(cameras: Query<&Camera>, cameras2d: Query<&Camera2d>) {
    info!("cameras : {:?}", cameras.iter().collect::<Vec<_>>());
    info!("cameras2d : {:?}", cameras2d.iter().count());
}
/// set up a simple 2D scene
fn setup_2d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // commands.spawn((
    //     MaterialMesh2dBundle {
    //         mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
    //         transform: Transform::default().with_scale(Vec3::splat(128.)),
    //         material: materials.add(ColorMaterial::from(Color::PURPLE)),
    //         ..default()
    //     },
    //     PickableBundle::default(), // <- Makes the mesh pickable.
    // ));
    // // camera
    commands.spawn((Camera2dBundle::default(), PickingCameraBundle::default()));
}
