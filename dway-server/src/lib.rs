#![feature(option_get_or_insert_default)]
#![feature(ptr_metadata)]
#![feature(trivial_bounds)]
use std::process::{self, Stdio};

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use schedule::DWayStartSet;
use state::{create_display, DWayWrapper, DisplayCreated, NonSendMark};
pub mod client;
pub mod dispatch;
pub mod display;
pub mod eventloop;
pub mod events;
pub mod geometry;
pub mod input;
pub mod macros;
mod prelude;
pub mod render;
pub mod resource;
pub mod schedule;
pub mod state;
pub mod util;
pub mod wl;
pub mod wp;
pub mod x11;
pub mod xdg;
pub mod zxdg;

#[derive(Default)]
pub struct DWayServerPlugin;
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(bevy_tokio_tasks::TokioTasksPlugin::default());
        app.add_plugin(state::DWayStatePlugin);
        app.add_plugin(geometry::GeometryPlugin);
        app.add_plugin(schedule::DWayServerSchedulePlugin);
        app.add_plugin(events::EventPlugin);
        app.add_plugin(wl::output::WlOutputPlugin);
        app.add_plugin(wl::surface::WlSurfacePlugin);
        app.add_plugin(wl::buffer::WlBufferPlugin);
        app.add_plugin(wl::region::WlRegionPlugin);
        app.add_plugin(wl::compositor::WlCompositorPlugin);
        app.add_plugin(input::seat::WlSeatPlugin);
        app.add_plugin(render::DWayServerRenderPlugin);
        app.add_plugin(xdg::XdgShellPlugin);
        app.add_plugin(xdg::toplevel::XdgToplevelPlugin);
        app.add_plugin(xdg::popup::XdgPopupPlugin);
        app.add_plugin(zxdg::outputmanager::XdgOutputManagerPlugin);
        app.add_plugin(wp::PrimarySelectionPlugin);
        app.add_startup_systems((init_display, apply_system_buffers, spawn).chain());
    }
}
pub fn init_display(
    _: NonSend<NonSendMark>,
    mut commands: Commands,
    mut event_sender: EventWriter<DisplayCreated>,
) {
    let entity = create_display(&mut commands, &mut event_sender);
    commands.entity(entity).log_components();
}
pub fn spawn(query: Query<&DWayWrapper>,tokio:Res<TokioTasksRuntime>) {
    info!("spawn app");
    let compositor = query.single().0.lock().unwrap();
    let mut command = process::Command::new("gnome-calculator");
    let mut command = process::Command::new("gedit");
    let mut command = process::Command::new("gnome-system-monitor");
    // let mut command = process::Command::new(
    // "/home/wangzi/workspace/waylandcompositor/conrod/target/debug/examples/all_winit_glium",
    // );
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
    compositor.spawn_process(command,&tokio);
}
