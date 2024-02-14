//! Demonstrates the use of [`UiMaterials`](UiMaterial) and how to change material values

use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use dway_ui_framework::shader::effect::Border;
use dway_ui_framework::shader::effect::Shadow;
use dway_ui_framework::shader::fill::FillColor;
use dway_ui_framework::shader::fill::Gradient;
use dway_ui_framework::shader::shape::Circle;
use dway_ui_framework::shader::shape::Shape;
use dway_ui_framework::shader::shape::*;
use dway_ui_framework::shader::{ShaderAsset, ShaderPlugin, ShapeRender};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShaderPlugin::<ButtonStyle>::default())
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

type ButtonStyle = ShapeRender<Circle, (Border, Gradient, Shadow)>;
fn circle_button_shader() -> ButtonStyle {
    ShapeRender::new(
        Circle::new(),
        // RoundedRect::new(Vec2::new(250.0, 125.0), 16.0),
        (
            Border::new(Color::WHITE, 2.0),
            // FillColor::new(Color::BLUE),
            Gradient::new(
                Color::WHITE * 0.5,
                Color::BLUE.rgba_to_vec4() - Color::RED.rgba_to_vec4(),
                Vec2::ONE.normalize() / 256.0,
            ),
            Shadow::new(Color::BLUE, Vec2::new(0.0, 0.0), Vec2::new(4.0, 4.0), 4.0),
        ),
    )
}

fn setup(mut commands: Commands, mut ui_materials: ResMut<Assets<ShaderAsset<ButtonStyle>>>) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            let spawn = parent.spawn(MaterialNodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    ..default()
                },
                material: ui_materials.add(circle_button_shader()),
                ..default()
            });
        });
}
