use std::path::Path;

use bevy::{render::RenderPlugin, ui::UiPlugin};
use lazy_static::lazy_static;
use pretty_assertions::assert_eq;
use regex::Regex;

use self::{
    effect::{Border, Shadow},
    fill::{FillColor, FillImage, Gradient},
    shape::{Circle, Rect, RoundedRect},
    transform::Translation,
};
use super::*;
use crate::tests::{run_test_plugins, UnitTestPlugin};

lazy_static! {
    static ref RE: Regex = Regex::new(r"  +").unwrap();
}

fn simplify_wgsl(input: &str) -> String {
    let input = input.replace(|c: char| c.is_whitespace(), " ");
    RE.replace_all(&input, " ").to_string()
}

fn test_render_type<R: Material>(except_wgsl: &str) {
    let mut app = App::default();
    app.add_plugins(
        MinimalPlugins
            .build()
            .add(AssetPlugin::default())
            .add(RenderPlugin::default())
            .add(UiPlugin),
    );
    let plugin = ShaderPlugin::<R>::default();
    app.add_plugins(plugin);
    let path = ShaderAsset::<R>::path();
    let _asset_path = ShaderRef::Path(path.clone().into());
    let wgsl = ShaderAsset::<R>::to_wgsl();
    assert_eq!(simplify_wgsl(&wgsl), simplify_wgsl(except_wgsl))
}

#[test]
fn generate_shader_shape() {
    test_render_type::<ShapeRender<RoundedRect, FillColor>>(
        r###"
#import bevy_render::view::View 
#import dway_ui_framework::shader::framework::sdf_visualition 
#import "embedded://bevy_ui_framework/shaders/framework.wgsl"::mix_alpha 
#import "embedded://bevy_ui_framework/shaders/framework.wgsl"::mix_color 
#import "embedded://bevy_ui_framework/shaders/framework.wgsl"::rounded_rect_sdf 
@group(0) @binding(0) var<uniform> view: View; 
@group(1) @binding(0) var<uniform> uniforms: Settings; 
struct Settings { @location(0) shape_radius: f32, @location(1) effect_color: vec4<f32>, } 
struct VertexOutput { @location(0) uv: vec2<f32>, @location(1) border_widths: vec4<f32>, @location(2) @interpolate(flat) size: vec2<f32>, @builtin(position) position: vec4<f32>, }; 
@vertex fn vertex( @location(0) vertex_position: vec3<f32>, @location(1) vertex_uv: vec2<f32>, @location(2) size: vec2<f32>, @location(3) border_widths: vec4<f32>, ) -> VertexOutput { var out: VertexOutput; out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0); out.border_widths = border_widths; var rect_position = (vertex_uv - 0.5) * size; var rect_size = size; var extend_left = 0.0; var extend_right = 0.0; var extend_top = 0.0; var extend_bottom = 0.0; out.uv = vertex_uv; out.size = size; return out; } 
@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> { var out = vec4(1.0, 1.0, 1.0, 0.0); let rect_position = (in.uv - 0.5) * in.size; let rect_size = in.size; { let shape_pos = rect_position; let shape_size = rect_size; let shape_d = rounded_rect_sdf(shape_pos, shape_size, uniforms.shape_radius); if shape_d<0.5 { out = mix_alpha(out, mix_color(uniforms.effect_color, shape_d)); if out.a > 255.0/256.0 { return out; } } } return out; }
"###,
    );
}

#[test]
fn generate_shader_multi_effect() {
    test_render_type::<ShapeRender<RoundedRect, (Border, FillImage)>>("");
}

#[test]
fn generate_shader_all_effect() {
    test_render_type::<ShapeRender<RoundedRect, (Border, FillColor, Shadow, Shadow)>>("");
}

#[test]
fn generate_shader_multi_shape() {
    test_render_type::<(
        ShapeRender<RoundedRect, (Border, FillColor, Shadow)>,
        Transformed<ShapeRender<Circle, (FillColor, Shadow)>, Translation>,
    )>("");
}

fn shader_unit_test<R: Material + Send + Sync + 'static>(
    dir: &Path,
    name: &str,
    size: Vec2,
    shader: R,
) -> UnitTestPlugin {
    let mut test_output_dir = dir.to_owned();
    test_output_dir.push(name);
    std::fs::create_dir_all(&test_output_dir).unwrap();
    UnitTestPlugin {
        name: name.to_owned(),
        image_path: format!("test/comparison_image/shader/{name}.png").into(),
        image_size: size,
        plugin: Box::new(move |_, app| {
            app.add_plugins(ShaderPlugin::<R>::default());
        }),
        setup: Box::new(move |args| {
            let camera_entity = args.camera_entity;
            let shader = shader.clone();
            Box::new(IntoSystem::into_system(
                move |mut commands: Commands, mut ui_material: ResMut<Assets<ShaderAsset<R>>>| {
                    let handle = ui_material.add(shader.clone());
                    commands.spawn((
                        MaterialNodeBundle {
                            style: Style {
                                width: Val::Px(256.),
                                height: Val::Px(256.),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                flex_direction: FlexDirection::Column,
                                align_self: AlignSelf::Center,
                                justify_self: JustifySelf::Center,
                                ..default()
                            },
                            material: handle,
                            ..default()
                        },
                        TargetCamera(camera_entity),
                    ));
                },
            ))
        }),
        output_dir: test_output_dir,
    }
}

#[test]
fn test_shaders() {
    let test_suite_name = "dway_ui_framework_unit_test";
    let temp_dir = tempdir::TempDir::new(test_suite_name).unwrap();
    let temp_dir_path = temp_dir.into_path();
    std::fs::create_dir_all(&temp_dir_path).unwrap();
    info!("template folder: {temp_dir_path:?}");

    run_test_plugins(
        test_suite_name,
        vec![
            shader_unit_test(
                &temp_dir_path,
                "circle_gradient_border_shadow",
                Vec2::splat(384.0),
                Circle::new().with_effect((
                    Border::new(Color::WHITE, 2.0),
                    Gradient::new(
                        Color::linear_rgb(0.5, 0.5, 0.5),
                        Vec4::new(-1.0, 0.0, 1.0, 0.0),
                        Vec2::ONE.normalize() / 256.0,
                    ),
                    Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0),
                )),
            ),
            shader_unit_test(
                &temp_dir_path,
                "rect_fill",
                Vec2::splat(384.0),
                Rect::new().with_effect(FillColor::new(Color::rgba(0.0, 0.0, 1.0, 1.0))),
            ),
        ],
    );

    std::fs::remove_dir_all(temp_dir_path).unwrap();
}
