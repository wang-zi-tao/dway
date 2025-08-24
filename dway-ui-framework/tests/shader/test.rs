use bevy::prelude::*;
use dway_ui_framework::{
    prelude::*,
    shader::{
        self,
        effect::*,
        fill::*,
        shape::{Circle, Rect, *},
        ShaderAsset, ShaderPlugin,
    },
};

#[path = "../common.rs"]
mod common;
use common::*;

pub fn unit_test<M: shader::Material + Clone>(app: &mut App, name: &str, material: M) {
    app.add_plugins(ShaderPlugin::<M>::default());
    let systemid = app.register_system(move|params: In<UnitTestParams>, mut commands: Commands, mut assets: ResMut<Assets<ShaderAsset< M >>>|{
        let handle = assets.add(ShaderAsset::new( material.clone() ));
        commands.spawn(( 
            style!("w-256 h-192 align-items:center justify-content:center align-self:center justify-self:center"),
            MaterialNode(handle),
            UiTargetCamera(params.camera)
        ));
    });

    app.world_mut().spawn(UnitTest {
        name: name.to_string(),
        image_path: format!("tests/shader/{name}.png").into(),
        image_size: Vec2::splat(384.0),
        setup: systemid,
    });
}

pub fn shapes_unit_test(name: &str, effect: impl Effect + Clone) {
    let mut app = App::new();

    app.add_plugins(TestPluginsSet)
        .insert_resource(TestSuite::new("test_shader"));

    unit_test(
        &mut app,
        &format!("circle_{name}"),
        Circle::new().with_effect(effect.clone()),
    );

    unit_test(
        &mut app,
        &format!("rect_{name}"),
        Rect::new().with_effect(effect.clone()),
    );

    unit_test(
        &mut app,
        &format!("rounded_rect_{name}"),
        RoundedRect::new(32.0).with_effect(effect.clone()),
    );

    unit_test(
        &mut app,
        &format!("rounded_bar_{name}"),
        RoundedBar::new().with_effect(effect.clone()),
    );

    let exit = app.run();
    assert!(exit == AppExit::Success);
}

fn fill_color() -> Color {
    color!("#cccccc")
}
fn border_color() -> Color {
    color!("#0000ff")
}
fn shadow() -> Shadow {
    Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0)
}

#[test]
pub fn test_shader_fill() {
    shapes_unit_test("fill", FillColor::new(fill_color()));
}

#[test]
pub fn test_shader_fill_shadow() {
    shapes_unit_test("fill_shadow", (FillColor::new(fill_color()), shadow()));
}

#[test]
pub fn test_shader_border_fill() {
    shapes_unit_test(
        "border_fill",
        (
            Border::new(border_color(), 2.0),
            FillColor::new(fill_color()),
            shadow(),
        ),
    );
}

#[test]
pub fn test_shader_rainbow_fill() {
    shapes_unit_test(
        "rainbow_fill",
        (
            Border::with_filler(ColorWheel::default(), 2.0),
            FillColor::new(fill_color()),
        ),
    );
}

#[test]
pub fn test_shader_gradient_border_shadow() {
    shapes_unit_test(
        "gradient_border_shadow",
        (
            Border::new(Color::WHITE, 2.0),
            Gradient::new(
                Color::linear_rgb(0.5, 0.5, 0.5),
                Vec4::new(-1.0, 0.0, 1.0, 0.0),
                Vec2::ONE.normalize() / 256.0,
            ),
            shadow(),
        ),
    );
}
