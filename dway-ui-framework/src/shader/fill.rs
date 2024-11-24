use std::marker::PhantomData;

use bevy::render::render_resource::{
    encase::internal::{BufferMut, Writer},
    AsBindGroupError,
};

use super::{BuildBindGroup, Expr, ShaderBuilder};
use crate::{prelude::*, shader::ShaderVariables};

pub trait Fill: BuildBindGroup {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr;
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Gradient {
    pub color: LinearRgba,
    pub delta_color: Vec4,
    pub direction: Vec2,
}

impl Gradient {
    pub fn new(color: Color, delta_color: Vec4, direction: Vec2) -> Self {
        Self {
            color: color.to_linear(),
            delta_color,
            direction,
        }
    }
}
impl Fill for Gradient {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, .. } = var;
        let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
        let uniform_delta_color = builder.get_uniform("delta_color", "", "vec4<f32>");
        let uniform_direction = builder.get_uniform("direction", "", "vec2<f32>");
        format!("({uniform_color} + {uniform_delta_color} * dot({pos}, {uniform_direction}))")
    }
}
impl BuildBindGroup for Gradient {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.color);
        layout.update_layout(&self.delta_color);
        layout.update_layout(&self.direction);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.color, writer);
        layout.write_uniform(&self.delta_color, writer);
        layout.write_uniform(&self.direction, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct ColorWheel {}
impl ColorWheel {
    pub fn new() -> Self {
        Self {}
    }
}
impl BuildBindGroup for ColorWheel {
    fn update_layout(&self, _layout: &mut super::UniformLayout) {
    }

    fn write_uniform<B: BufferMut>(
        &self,
        _layout: &mut super::UniformLayout,
        _writer: &mut Writer<B>,
    ) {
    }
}
impl Fill for ColorWheel {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, .. } = var;
        builder.import_from_builtin("color_wheel");
        format!("color_wheel({pos})")
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct FillColor {
    pub color: LinearRgba,
}

impl From<Color> for FillColor {
    fn from(value: Color) -> Self {
        Self {
            color: value.to_linear(),
        }
    }
}

impl FillColor {
    pub fn new(color: Color) -> Self {
        Self {
            color: color.to_linear(),
        }
    }
}
impl Fill for FillColor {
    fn to_wgsl(builder: &mut ShaderBuilder, _pos: &ShaderVariables) -> Expr {
        builder.get_uniform("color", "", "vec4<f32>")
    }
}
impl BuildBindGroup for FillColor {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.color);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.color, writer);
    }
}

#[derive(Clone, Default, Debug)]
pub struct FillImage {
    pub offset: Vec2,
    pub scaling: Vec2,
    pub image: Handle<Image>,
}
impl From<Handle<Image>> for FillImage {
    fn from(value: Handle<Image>) -> Self {
        Self {
            offset: Vec2::ZERO,
            scaling: Vec2::ONE,
            image: value,
        }
    }
}
impl FillImage {
    pub fn new(offset: Vec2, scaling: Vec2, image: Handle<Image>) -> Self {
        Self {
            offset,
            scaling,
            image,
        }
    }
}
impl Fill for FillImage {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let uniform_offset = builder.get_uniform("offset_uv", "", "vec2<f32>");
        let uniform_scaling = builder.get_uniform("scaling", "", "vec2<f32>");
        let var_image_texture = builder.get_binding("image_texture", "", "texture_2d<f32>");
        let var_image_sampler = builder.get_binding("image_sampler", "", "sampler");
        format!("textureSample({var_image_texture}, {var_image_sampler}, ({pos} + 0.5 * {size} - {uniform_offset}*{size})/({uniform_scaling}*{size}))")
    }
}
impl BuildBindGroup for FillImage {
    fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {
        builder.add_image();
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut super::BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        builder.add_image(&self.image)?;
        Ok(())
    }

    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.offset);
        layout.update_layout(&self.scaling);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.offset, writer);
        layout.write_uniform(&self.scaling, writer);
    }
}

pub trait BlurMethod: Clone + Send + Sync + 'static {
    fn method() -> String;
}
#[derive(Clone)]
pub struct KawaseBlur;
impl BlurMethod for KawaseBlur {
    fn method() -> String {
        "kawase_blur_image".to_string()
    }
}
#[derive(Clone)]
pub struct KawaseLevel2Blur;
impl BlurMethod for KawaseLevel2Blur {
    fn method() -> String {
        "kawase_blur_image2".to_string()
    }
}
#[derive(Clone)]
pub struct GaussianBlur;
impl BlurMethod for GaussianBlur {
    fn method() -> String {
        "gaussian_blur_image5".to_string()
    }
}

#[derive(Clone, SmartDefault, Debug)]
pub struct BlurImage<M: BlurMethod> {
    #[default(1.0)]
    pub radius: f32,
    pub image: Handle<Image>,
    phantom: PhantomData<M>,
}
impl<M: BlurMethod> BlurImage<M> {
    pub fn new(radius: f32, image: Handle<Image>) -> Self {
        Self {
            radius,
            image,
            phantom: PhantomData,
        }
    }
}
impl<M: BlurMethod> Fill for BlurImage<M> {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let blur_method = M::method();
        builder.import_from_builtin(&blur_method);
        let uniform_radius = builder.get_uniform("radius", "", "f32");
        let var_image_texture = builder.get_binding("image_texture", "", "texture_2d<f32>");
        let var_image_sampler = builder.get_binding("image_sampler", "", "sampler");
        format!("{blur_method}({var_image_texture}, {var_image_sampler}, {pos}, {size}, {uniform_radius})")
    }
}
impl<M: BlurMethod> BuildBindGroup for BlurImage<M> {
    fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {
        builder.add_image();
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut super::BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        builder.add_image(&self.image)?;
        Ok(())
    }

    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.radius);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.radius, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct AddColor<F: Fill> {
    pub inner: F,
    pub color: LinearRgba,
}

impl<F: Fill> AddColor<F> {
    pub fn new(inner: F, color: Color) -> Self {
        Self {
            inner,
            color: color.to_linear(),
        }
    }
}

impl<F: Fill> Fill for AddColor<F> {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let inner = F::to_wgsl(builder, var);
        let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
        builder.import_from_builtin("mix_alpha");
        format!("mix_alpha({inner}, {uniform_color})")
    }
}
impl<F: Fill> BuildBindGroup for AddColor<F> {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        self.inner.update_layout(layout);
        layout.update_layout(&self.color);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        self.inner.write_uniform(layout, writer);
        layout.write_uniform(&self.color, writer);
    }

    fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {
        F::bind_group_layout_entries(builder);
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut super::BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        self.inner.unprepared_bind_group(builder)
    }
}
