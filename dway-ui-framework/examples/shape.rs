use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use bevy_prototype_lyon::{
    draw::{Fill, Stroke},
    entity::Path,
    path::PathBuilder,
};
use bevy_svg::prelude::{FillOptions, StrokeOptions};
use chrono::Timelike;
use dway_ui_derive::color;
use dway_ui_framework::{widgets::shape::UiShapeBundle, UiFrameworkPlugin};
use std::{f32::consts::PI, time::Duration};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            UiFrameworkPlugin,
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin {
                wait_duration: Duration::from_secs(4),
                ..Default::default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

#[derive(Component)]
pub struct Clock;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        Clock,
        Fill {
            options: FillOptions::default(),
            color: color!("#ffff00"),
        },
        Stroke {
            options: StrokeOptions::default()
                .with_line_join(bevy_svg::prelude::LineJoin::Round)
                .with_end_cap(bevy_svg::prelude::LineCap::Round)
                .with_start_cap(bevy_svg::prelude::LineCap::Round)
                .with_line_width(8.0),
            color: Color::BLACK,
        },
        UiShapeBundle {
            style: Style {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                width: Val::Px(256.0),
                height: Val::Px(256.0),
                ..default()
            },
            ..default()
        },
    ));
}
fn update(mut query: Query<&mut Path, With<Clock>>) {
    for mut path in &mut query {
        let time = chrono::offset::Local::now();
        let h = time.hour12().1 as f32;
        let m = time.minute() as f32;
        let s = time.second() as f32 + time.timestamp_subsec_millis() as f32 / 1000.0;

        let polar2rectangular = |angle: f32, l: f32| Vec2::new(angle.sin(), angle.cos()) * l;

        let mut builder = PathBuilder::new();

        builder.move_to(Vec2::Y * 160.0);
        builder.arc(Vec2::ZERO, Vec2::splat(160.0), 2. * PI, 1.0);
        for i in 0..12 {
            builder.move_to(polar2rectangular(i as f32 * 2. * PI / 12., 144.0));
            builder.line_to(polar2rectangular(i as f32 * 2. * PI / 12., 128.0));
        }
        builder.move_to(polar2rectangular(h * 2. * PI / 60.0, 64.0));
        builder.line_to(Vec2::ZERO);
        builder.move_to(polar2rectangular(m * 2. * PI / 60.0, 96.0));
        builder.line_to(Vec2::ZERO);
        builder.move_to(polar2rectangular(s * 2. * PI / 60.0, 128.0));
        builder.line_to(Vec2::ZERO);

        *path = builder.build();
    }
}
