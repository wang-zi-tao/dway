use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use dway_ui_derive::color;
use dway_ui_framework::shader::{
    effect::{Border, Shadow},
    fill::{FillColor, Gradient},
    shape::{Circle, *},
    transform::Margins,
    ShaderAsset, ShaderPlugin, ShapeRender, Transformed,
};

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
    commands.spawn((Camera2d::default(), Msaa::Sample4));

    let node = Node {
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
        color!("#808080"),
        Vec4::new(-1.0, 0.0, 1.0, 0.0),
        Vec2::ONE.normalize() / 256.0,
    );

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
            commands
                .spawn(NodeBundle {
                    node: Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(MaterialNodeBundle {
                        node: Node {
                            width: Val::Px(250.0),
                            height: Val::Px(250.0),
                            margin: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        material: ui_materials.add(Circle::new().with_effect((
                            Border::new(Color::WHITE, 2.0),
                            gradient.clone(),
                            shadow.clone(),
                        ))).into(),
                        ..default()
                    });
                    parent
                        .spawn(MaterialNodeBundle {
                            node: Node {
                                margin: UiRect::all(Val::Px(8.0)),
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            material: ui_material_button
                                .add(
                                    RoundedRect::new(8.0)
                                        .with_effect(effect_border_fill_shadow.clone()),
                                )
                                .into(),
                            ..default()
                        })
                        .with_child((
                            Text::new("Button"),
                            TextFont::from_font_size(24.0),
                            TextColor(ui_color),
                        ));
                    parent
                        .spawn(MaterialNodeBundle {
                            node: Node {
                                margin: UiRect::all(Val::Px(8.0)),
                                padding: UiRect::all(Val::Px(4.0)),
                                ..default()
                            },
                            material: ui_material_button
                                .add(RoundedRect::new(8.0).with_effect((
                                    Border::new(Color::WHITE, 0.0),
                                    blue.into(),
                                    shadow.clone(),
                                )))
                                .into(),
                            ..default()
                        })
                        .with_child((
                            Text::new("Button"),
                            TextFont::from_font_size(24.0),
                            TextColor(Color::WHITE),
                        ));
                    parent.spawn(MaterialNodeBundle {
                        node: node.clone(),
                        material: ui_material_checkbox
                            .add((
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
                            ))
                            .into(),
                        ..default()
                    });
                });
        });
}
