use std::default;

use bevy::prelude::*;
// use bevy_mod_picking::{
//     DebugCursorPickingPlugin, DebugEventsPickingPlugin, DefaultPickingPlugins, PickingCameraBundle,
// };
use dway_server::DWayServerSystem;
use log::info;

use crate::window::{Backend, Frontends, WindowUiRoot};

pub mod components;
pub mod compositor;
pub mod debug;
pub mod desktop;
pub mod input;
pub mod moving;
pub mod protocol;
pub mod render;
pub mod resizing;
pub mod screen;
pub mod window;
pub mod workspace;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum DWayClientSystem {
    FromServer,
    Create,
    CreateFlush,
    CreateComponent,
    CreateComponentFlush,
    Input,
    UpdateState,
    UpdateFocus,
    UpdateUI,
    PostUpdate,
    DestroyComponent,
    Destroy,
    DestroyFlush,
    ToServer,
}

#[derive(Hash, Default, Debug, PartialEq, Eq, Clone, States)]
pub enum DWayClientState {
    Init,
    #[default]
    Desktop,
    Locked,
    Overview,
    Fullscreen,
    Moving,
    Resizing,
    Eixt,
}

pub struct WaylandPlugin;
impl Plugin for WaylandPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_state::<DWayClientState>();
        use DWayClientSystem::*;
        app.configure_sets(
            (
                FromServer.after(DWayServerSystem::Update),
                Create,
                CreateFlush,
                CreateComponent,
                CreateComponentFlush,
                Input,
                UpdateState,
                UpdateFocus,
                UpdateUI,
            )
                .in_base_set(CoreSet::PreUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (
                PostUpdate,
                DestroyComponent,
                Destroy,
                DestroyFlush,
                ToServer.before(DWayServerSystem::PostUpdate),
            )
                .in_base_set(CoreSet::PostUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.add_system(apply_system_buffers.in_set(CreateFlush));
        app.add_system(apply_system_buffers.in_set(CreateComponentFlush));
        app.add_system(apply_system_buffers.in_set(DestroyFlush));

        app.add_plugin(compositor::CompositorPlugin);
        // app.add_plugin(DebugCursorPickingPlugin);
        // app.add_plugin(DebugEventsPickingPlugin);
        // app.add_plugins(DefaultPickingPlugins);
        app.add_startup_system(setup_2d);
        app.add_plugin(input::DWayInputPlugin { debug: false });
        app.add_plugin(desktop::DWayDesktop);
        app.add_plugin(window::DWayWindowPlugin);
        app.add_plugin(moving::DWayMovingPlugin::default());
        app.add_plugin(resizing::DWayResizingPlugin::default());
        app.add_plugin(debug::DebugPlugin::default());
        // app.add_system(debug_info);
        //
        app.register_type::<Backend>();
        app.register_type::<Frontends>();
        app.register_type::<WindowUiRoot>();
    }
}

pub fn debug_info(cameras: Query<&Camera>, cameras2d: Query<&Camera2d>) {
    info!("cameras : {:?}", cameras.iter().collect::<Vec<_>>());
    info!("cameras2d : {:?}", cameras2d.iter().count());
}
/// set up a simple 2D scene
fn setup_2d(
    mut commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<ColorMaterial>>,
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
    // let camera=Camera2dBundle::default();
    // camera.camera.priority=0;
    // camera.camera_2d.clear_color=ClearColorConfig::None;
    // camera.transform.translation.z=1024.0;
    commands.spawn(Camera2dBundle::default());
    // commands.spawn((camera, PickingCameraBundle::default()));
}
