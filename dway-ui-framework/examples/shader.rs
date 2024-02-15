use std::time::Duration;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use dway_ui_derive::color;
use dway_ui_framework::shader::effect::Border;
use dway_ui_framework::shader::effect::Shadow;
use dway_ui_framework::shader::fill::FillColor;
use dway_ui_framework::shader::fill::Gradient;
use dway_ui_framework::shader::shape::Circle;
use dway_ui_framework::shader::shape::Shape;
use dway_ui_framework::shader::shape::*;
use dway_ui_framework::shader::transform::Margins;
use dway_ui_framework::shader::transform::Translation;
use dway_ui_framework::shader::Transformed;
use dway_ui_framework::shader::{ShaderAsset, ShaderPlugin, ShapeRender};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(dway_ui_framework::UiFrameworkPlugin)
        .add_plugins(ShaderPlugin::<CircleStyle>::default())
        .add_plugins(ShaderPlugin::<ButtonStyle>::default())
        .add_plugins(ShaderPlugin::<CheckboxStyle>::default())
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
        ))
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
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
    mut ui_materials: ResMut<Assets<ShaderAsset<CircleStyle>>>,
    mut ui_material_button: ResMut<Assets<ShaderAsset<ButtonStyle>>>,
    mut ui_material_checkbox: ResMut<Assets<ShaderAsset<CheckboxStyle>>>,
) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    let style = Style {
        width: Val::Px(64.0),
        height: Val::Px(32.0),
        margin: UiRect::all(Val::Px(8.0)),
        ..default()
    };

    let blue = color!("#3050e0");
    let ui_color = color!("#484E5B");

    let shadow = Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0);
    let effect_border_fill_shadow = (
        Border::new(ui_color, 3.0),
        Color::WHITE.into(),
        shadow.clone(),
    );
    let gradient = Gradient::new(
        Color::WHITE * 0.5,
        Color::BLUE.rgba_to_vec4() - Color::RED.rgba_to_vec4(),
        Vec2::ONE.normalize() / 256.0,
    );

    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            commands
                .spawn(NodeBundle {
                    style: Style {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(MaterialNodeBundle {
                        style: Style {
                            width: Val::Px(250.0),
                            height: Val::Px(250.0),
                            margin: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        material: ui_materials.add(Circle::new().with_effect((
                            Border::new(Color::WHITE, 2.0),
                            gradient.clone(),
                            shadow.clone(),
                        ))),
                        ..default()
                    });
                    parent
                        .spawn(MaterialNodeBundle {
                            style: (Style {
                                margin: UiRect::all(Val::Px(8.0)),
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            }),
                            material: ui_material_button.add(
                                RoundedRect::new(8.0)
                                    .with_effect(effect_border_fill_shadow.clone()),
                            ),
                            ..default()
                        })
                        .with_children(|c| {
                            c.spawn(TextBundle {
                                text: Text::from_section(
                                    "Button",
                                    TextStyle {
                                        font: Default::default(),
                                        font_size: 24.0,
                                        color: ui_color,
                                    },
                                ),
                                ..Default::default()
                            });
                        });
                    parent
                        .spawn(MaterialNodeBundle {
                            style: (Style {
                                margin: UiRect::all(Val::Px(8.0)),
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            }),
                            material: ui_material_button.add(RoundedRect::new(8.0).with_effect((
                                Border::new(Color::WHITE, 0.0),
                                blue.into(),
                                shadow.clone(),
                            ))),
                            ..default()
                        })
                        .with_children(|c| {
                            c.spawn(TextBundle {
                                text: Text::from_section(
                                    "Button",
                                    TextStyle {
                                        font: Default::default(),
                                        font_size: 24.0,
                                        color: Color::WHITE,
                                    },
                                ),
                                ..Default::default()
                            });
                        });
                    parent.spawn(MaterialNodeBundle {
                        style: style.clone(),
                        material: ui_material_checkbox.add((
                            Circle::default()
                                .with_effect((
                                    Border::new(ui_color, 2.0),
                                    ui_color.into(),
                                    shadow.clone(),
                                ))
                                .with_transform(Margins::new(5.0, 32.0 + 5.0, 5.0, 5.0)),
                            RoundedBar::default().with_effect((
                                Border::new(ui_color, 3.0),
                                Color::WHITE.into(),
                                shadow.clone(),
                            )),
                        )),
                        ..default()
                    });
                });
        });
}
