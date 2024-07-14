use bevy::asset::io::embedded::EmbeddedAssetRegistry;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use dway_ui_framework::prelude::*;
use std::path::PathBuf;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins((
            dway_ui_framework::UiFrameworkPlugin,
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE));

    {
        let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        embedded.insert_asset(std::path::PathBuf::new(), &PathBuf::from("dway_ui_framework/examples/gallary/power.svg"), r###"
<svg xmlns="http://www.w3.org/2000/svg" height="24px" viewBox="0 0 24 24" width="24px" fill="#000000"><path d="M0 0h24v24H0V0z" fill="none"/><path d="M13 3h-2v10h2V3zm4.83 2.17l-1.42 1.42C17.99 7.86 19 9.81 19 12c0 3.87-3.13 7-7 7s-7-3.13-7-7c0-2.19 1.01-4.14 2.58-5.42L6.17 5.17C4.23 6.82 3 9.26 3 12c0 4.97 4.03 9 9 9s9-4.03 9-9c0-2.74-1.23-5.18-3.17-6.83z"/></svg>
"###.to_string().into_bytes());
    }

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn(UiSvgBundle {
        svg: UiSvg::from(
            asset_server.load("embedded://dway_ui_framework/examples/gallary/power.svg"),
        ),
        style: style!("w-96 h-96"),
        ..default()
    });
}
