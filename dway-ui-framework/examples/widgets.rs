use bevy::prelude::*;
use dway_ui_derive::style;
use dway_ui_framework::{
    theme::Theme,
    widgets::{
        bundles::{
            UiBlockBundle, UiHighlightBlockBundle, UiHollowBlockBundle, UiNodeBundle,
            UiSunkenBlockBundle,
        },
        button::UiButtonBundle,
        checkbox::UiCheckboxBundle,
        inputbox::UiInputBoxBundle,
        slider::UiSliderBundle,
        text::UiTextBundle,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((dway_ui_framework::UiFrameworkPlugin,))
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE * 0.8))
        .add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new())
        .run();
}

fn setup(mut commands: Commands, theme: Res<Theme>) {
    // Camera so we can see UI
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(UiNodeBundle {
            style: Style {
                align_self: AlignSelf::Center,
                justify_self: JustifySelf::Center,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..style!("p-8")
            },
            ..default()
        })
        .with_children(|c| {
            c.spawn(UiBlockBundle {
                style: Style {
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..style!("w-256 h-256 p-8 m-8")
                },
                ..default()
            })
            .with_children(|c| {
                c.spawn(UiButtonBundle {
                    style: style!("w-64 h-32 m-8 align-items:center justify-content:center"),
                    ..Default::default()
                })
                .with_children(|c| {
                    c.spawn(UiTextBundle::new("button", 24, &theme));
                });
                c.spawn(UiCheckboxBundle {
                    style: style!("w-64 h-32 m-8"),
                    ..Default::default()
                });
                c.spawn(UiSliderBundle {
                    style: style!("w-128 h-32 m-8"),
                    ..Default::default()
                });
                c.spawn((UiInputBoxBundle {
                    style: style!("w-128 h-32"),
                    ..Default::default()
                },));
            });
            c.spawn(UiHollowBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            })
            .with_children(|c| {
                c.spawn(UiInputBoxBundle {
                    style: style!("full m-8"),
                    ..Default::default()
                });
            });
            c.spawn(UiSunkenBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            });
            c.spawn(UiHighlightBlockBundle {
                style: style!("w-256 h-256 p-8 m-8"),
                ..Default::default()
            });
        });
}
