use std::process;

use bevy::prelude::*;
use dway_server::{
    state::{DWayServer, WaylandDisplayCreated},
    x11::DWayXWaylandReady,
};

pub fn spawn(
    mut events: EventReader<WaylandDisplayCreated>,
    query: Query<&DWayServer, Added<DWayServer>>,
) {
    for WaylandDisplayCreated(dway_entity, _) in events.read() {
        if let Ok(compositor) = query.get(*dway_entity) {
            for _i in 0..1 {
                let command = process::Command::new("alacritty");
                compositor.spawn_process(command);
            }

            for command in [
                "tilix",
                "gnome-system-monitor",
                "gedit",
                "gnome-calculator",
                "gnome-clocks",
                "gnome-disks",
                "gnome-logs",
                "gnome-music",
                "gnome-maps",
                "gnome-photos",
                "gnome-text-editor",
                "gnome-tweaks",
                "gnome-weather",
                // "/home/wangzi/.build/5e0dff7f0473a25a4eb0bbaeeda9b3fa091ba89-wgpu/debug/examples/cube",
                "alacritty",
            ] {
                compositor.spawn_process(process::Command::new(command));
            }

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
            // compositor.spawn_process(command);
        }
    }
}
pub fn spawn_x11(query: Query<&DWayServer>, mut events: EventReader<DWayXWaylandReady>) {
    for DWayXWaylandReady { dway_entity } in events.read() {
        if let Ok(_compositor) = query.get(*dway_entity) {
            // compositor.spawn_process(process::Command::new("glxgears"));
            // compositor.spawn_process_x11(process::Command::new("/mnt/weed/mount/wangzi-nuc/wangzi/workspace/waylandcompositor/source/gtk+-3.24.37/build/examples/sunny"));
            // compositor.spawn_process_x11(process::Command::new("gnome-system-monitor"));
        }
    }
}
