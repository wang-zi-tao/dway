#![feature(option_get_or_insert_default)]
#![feature(async_closure)]
#![feature(ptr_metadata)]
#![feature(trivial_bounds)]
#![feature(iterator_try_collect)]

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use dway_util::eventloop::EventLoop;
use schedule::DWayServerSet;
use state::{create_display, WaylandDisplayCreated, DWayServerConfig, DWayServer};
use std::process;
use x11::DWayXWaylandReady;
pub mod apps;
pub mod client;
pub mod dispatch;
pub mod display;
pub mod events;
pub mod geometry;
pub mod input;
pub mod macros;
pub mod prelude;
pub mod render;
pub mod resource;
pub mod schedule;
pub mod state;
pub mod util;
pub mod wl;
pub mod wp;
pub mod x11;
pub mod xdg;
pub mod zwp;
pub mod zxdg;

#[derive(Default)]
pub struct DWayServerPlugin;
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy_tokio_tasks::TokioTasksPlugin::default(),
            state::DWayStatePlugin,
            client::ClientPlugin,
            geometry::GeometryPlugin,
            schedule::DWayServerSchedulePlugin,
            events::EventPlugin,
            render::DWayServerRenderPlugin,
        ));
        app.add_plugins((
            wl::output::WlOutputPlugin,
            wl::surface::WlSurfacePlugin,
            wl::buffer::WlBufferPlugin,
            wl::region::WlRegionPlugin,
            wl::compositor::WlCompositorPlugin,
            xdg::XdgShellPlugin,
            xdg::toplevel::XdgToplevelPlugin,
            xdg::popup::XdgPopupPlugin,
            zxdg::outputmanager::XdgOutputManagerPlugin,
            input::seat::WlSeatPlugin,
        ));
        app.add_plugins((
            wp::PrimarySelectionPlugin,
            x11::DWayXWaylandPlugin,
            zwp::DmaBufferPlugin,
            apps::DesktopEntriesPlugin,
        ));
        app.add_systems(Startup, (init_display, apply_deferred, spawn).chain());
        app.add_systems(
            PreUpdate,
            spawn_x11
                .run_if(on_event::<DWayXWaylandReady>())
                .in_set(DWayServerSet::UpdateXWayland),
        );
    }
}
pub fn init_display(
    mut commands: Commands,
    mut event_sender: EventWriter<WaylandDisplayCreated>,
    config: Res<DWayServerConfig>,
    mut event_loop: NonSendMut<EventLoop>,
) {
    create_display(&mut commands, &config, &mut event_sender, &mut event_loop);
}
pub fn spawn(query: Query<&DWayServer>, tokio: Res<TokioTasksRuntime>) {
    let compositor = query.single();
    compositor.spawn_process(process::Command::new("gnome-calculator"), &tokio);
    compositor.spawn_process(process::Command::new("gedit"), &tokio);
    compositor.spawn_process(process::Command::new("gnome-system-monitor"), &tokio);
    compositor.spawn_process(
        process::Command::new(
            "/home/wangzi/.build/5e0dff7f0473a25a4eb0bbaeeda9b3fa091ba89-wgpu/debug/examples/cube",
        ),
        &tokio,
    );
    // let mut command = process::Command::new("alacritty");
    // command.args(["-e", "htop"]);
    // command.current_dir("/home/wangzi/workspace/waylandcompositor/conrod/");
    // let mut command = process::Command::new("/nix/store/gfn9ya0rwaffhfkpbbc3pynk247xap1h-qt5ct-1.5/bin/qt5ct");
    // let mut command = process::Command::new("/home/wangzi/.build/0bd4966a8a745859d01236fd5f997041598cc31-bevy/debug/examples/animated_transform");
    // let mut command = process::Command::new( "/home/wangzi/workspace/waylandcompositor/winit_demo/target/debug/winit_demo",);
    // let mut command = process::Command::new("/home/wangzi/workspace/waylandcompositor/wayland-rs/wayland-client/../target/debug/examples/simple_window");
    // let mut command = process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/GTK-Demo-Examples/guidemo/00_hello_world_classic/hello_world_classic");
    // let mut command =
    //     process::Command::new("/home/wangzi/Code/winit/target/debug/examples/window_debug");
    // compositor.spawn_process(command, &tokio);
}
pub fn spawn_x11(
    query: Query<&DWayServer>,
    tokio: Res<TokioTasksRuntime>,
    mut events: EventReader<DWayXWaylandReady>,
) {
    for DWayXWaylandReady { dway_entity } in events.iter() {
        if let Ok(compositor) = query.get(*dway_entity) {
            // compositor.spawn_process(process::Command::new("glxgears"), &tokio);
            // compositor.spawn_process_x11(process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/source/gtk+-3.24.37/build/examples/sunny"), &tokio);
            // compositor.spawn_process_x11(process::Command::new("gnome-system-monitor"), &tokio);
        }
    }
}
