use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use dway_ui_derive::color;
use dway_ui_framework::{prelude::UiHollowBlockBundle, shader::{
    effect::{Border, Shadow},
    fill::{FillColor, Gradient},
    shape::{Circle, *},
    transform::Margins,
    ShaderAsset, ShaderPlugin, ShapeRender, Transformed,
}, text::{cursor::UiTextCursor, editor::UiTextEditor, selection::UiTextSelection, textarea::UiTextArea}};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(dway_ui_framework::UiFrameworkPlugin)
        .add_plugins(ShaderPlugin::<CircleStyle>::default())
        .add_plugins(ShaderPlugin::<ButtonStyle>::default())
        .add_plugins(ShaderPlugin::<CheckboxStyle>::default())
        .add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
        ))
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .run();
}

type CircleStyle = ShapeRender<Circle, (Border, Gradient, Shadow)>;

type CheckboxStyle = (
    Transformed<ShapeRender<Circle, (Border, FillColor, Shadow)>, Margins>,
    ShapeRender<RoundedBar, (Border, FillColor, Shadow)>,
);

type ButtonStyle = ShapeRender<RoundedRect, (Border, FillColor, Shadow)>;

fn setup(
    mut commands: Commands,
) {
    // Camera so we can see UI
    commands.spawn((Camera2d::default(), Msaa::Sample4));

    dway_ui_derive::spawn!(&mut commands=>
        <Node @style="flex-col left-512 top-512">
            <UiHollowBlockBundle @style="w-256 h-32">
                <(UiTextArea::new("text area", 28.0)) @style="full"/>
            </UiHollowBlockBundle>
            <UiHollowBlockBundle @style="w-256 h-128">
                <(UiTextArea::new("text cursor\n text cursor\n text cursor\n text cursor", 28.0)) @style="full" 
                    UiTextCursor=(default()) />
            </UiHollowBlockBundle>
            <UiHollowBlockBundle @style="w-256 h-128">
                <(UiTextArea::new("text selection\ntext selection\ntext selection\ntext selection\n", 28.0))  @style="full"
                    UiTextCursor=(default())
                    UiTextSelection=(default()) />
            </UiHollowBlockBundle>
            <UiHollowBlockBundle @style="w-256 h-32">
                <(UiTextArea::new("text selection", 28.0))  @style="full"
                    UiTextCursor=(default())
                    UiTextSelection=(default())
                    UiTextEditor=(default())
                />
            </UiHollowBlockBundle>
        </Node>
    );
}

