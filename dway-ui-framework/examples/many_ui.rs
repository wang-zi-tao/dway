use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*, render::RenderPlugin,
};
use dway_ui_derive::color;
use dway_ui_framework::shader::{
    effect::{Border, Shadow},
    fill::FillColor,
    shape::{Circle, *},
    transform::Margins,
    ShaderAsset, ShaderPlugin, ShapeRender, Transformed,
};

// const NODE_COUNT: usize = 40000;
const NODE_COUNT: usize = 34000;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(dway_ui_framework::UiFrameworkPlugin)
        .add_plugins(ShaderPlugin::<CheckboxStyle>::default())
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(1),
                ..Default::default()
            },
        ))
        .insert_resource(bevy::winit::WinitSettings {
            focused_mode: bevy::winit::UpdateMode::Continuous,
            unfocused_mode: bevy::winit::UpdateMode::Continuous,
        })
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

type CheckboxStyle = (
    Transformed<ShapeRender<Circle, (Border, FillColor, Shadow)>, Margins>,
    ShapeRender<RoundedBar, (Border, FillColor, Shadow)>,
);

fn setup(
    mut commands: Commands,
    mut ui_material_checkbox: ResMut<Assets<ShaderAsset<CheckboxStyle>>>,
) {
    // Camera so we can see UI
    commands.spawn((Camera2d::default(), Msaa::Sample4));

    let ui_color = color!("#484E5B");
    let shadow = Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0);

    commands
        .spawn(NodeBundle {
            node: Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            // 8192(no batch) / 40000(batch) node at 60 fps
            commands.spawn(NodeBundle::default()).with_children(|c| {
                let shader = (
                    Transformed::new(
                        ShapeRender::new(
                            Circle::default(),
                            (Border::new(ui_color, 2.0), ui_color.into(), shadow.clone()),
                        ),
                        Margins::new(1.0, 4.0 + 1.0, 1.0, 1.0),
                    ),
                    ShapeRender::new(
                        RoundedBar::new(),
                        (
                            Border::new(ui_color, 3.0),
                            Color::WHITE.into(),
                            shadow.clone(),
                        ),
                    ),
                );
                let handle = ui_material_checkbox.add(shader);
                for _i in 0..200 {
                    c.spawn(NodeBundle {
                        node: Node {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|c| {
                        for _j in 0..NODE_COUNT / 200 {
                            c.spawn(MaterialNodeBundle {
                                node: Node {
                                    width: Val::Px(8.0),
                                    height: Val::Px(4.0),
                                    ..default()
                                },
                                material: handle.clone().into(),
                                ..default()
                            });
                        }
                    });
                }
            });
        });
}
