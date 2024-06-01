use super::{effect::Effect, BuildBindGroup, Expr, ShaderBuilder, ShaderVariables, ShapeRender};
use crate::prelude::*;

pub trait Shape: BuildBindGroup {
    fn register_uniforms(builder: &mut ShaderBuilder);
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr;
    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr;

    fn with_effect<E: Effect>(self, e: E) -> ShapeRender<Self, E> {
        ShapeRender::new(self, e)
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Circle {}

impl Circle {
    pub fn new() -> Self {
        Self {}
    }
}
impl Shape for Circle {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("circle_sdf");
        format!("circle_sdf({pos}, 0.5 * min({size}.x, {size}.y))")
    }

    fn register_uniforms(_builder: &mut ShaderBuilder) {}

    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("circle_sdf_gradient");
        format!("circle_sdf_gradient({pos}, 0.5 * min({size}.x, {size}.y))")
    }
}
impl BuildBindGroup for Circle {
    fn update_layout(&self, _layout: &mut super::UniformLayout) {}

    fn write_uniform<B: encase::internal::BufferMut>(
        &self,
        _layout: &mut super::UniformLayout,
        _writer: &mut encase::internal::Writer<B>,
    ) {
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Rect {}

impl Rect {
    pub fn new() -> Self {
        Self {}
    }
}
impl Shape for Rect {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("rect_sdf");
        format!("rect_sdf({pos}, {size})")
    }

    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("rect_sdf_gradient");
        format!("rect_sdf_gradient({pos}, {size})")
    }

    fn register_uniforms(builder: &mut ShaderBuilder) {
        builder.get_uniform("size", "", "vec2<f32>");
    }
}
impl BuildBindGroup for Rect {
    fn update_layout(&self, _layout: &mut super::UniformLayout) {}
    fn write_uniform<B: encase::internal::BufferMut>(
        &self,
        _layout: &mut super::UniformLayout,
        _writer: &mut encase::internal::Writer<B>,
    ) {
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct RoundedRect {
    pub corner: f32,
}

impl From<f32> for RoundedRect {
    fn from(value: f32) -> Self {
        Self { corner: value }
    }
}

impl RoundedRect {
    pub fn new(corner: f32) -> Self {
        Self { corner }
    }
}
impl Shape for RoundedRect {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let uniform_radius = builder.get_uniform("radius", "", "f32");
        builder.import_from_builtin("rounded_rect_sdf");
        format!("rounded_rect_sdf({pos}, {size}, {uniform_radius})")
    }

    fn register_uniforms(builder: &mut ShaderBuilder) {
        builder.get_uniform("radius", "", "f32");
    }

    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let uniform_radius = builder.get_uniform("radius", "", "f32");
        builder.import_from_builtin("rounded_rect_sdf_gradient");
        format!("rounded_rect_sdf_gradient({pos}, {size}, {uniform_radius})")
    }
}
impl BuildBindGroup for RoundedRect {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.corner);
    }

    fn write_uniform<B: encase::internal::BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut encase::internal::Writer<B>,
    ) {
        layout.write_uniform(&self.corner, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct RoundedBar {}

impl RoundedBar {
    pub fn new() -> Self {
        Self {}
    }
}
impl Shape for RoundedBar {
    fn register_uniforms(_builder: &mut ShaderBuilder) {}

    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("rounded_rect_sdf");
        format!("rounded_rect_sdf({pos}, {size}, 0.5 * min({size}.x, {size}.y))")
    }

    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("rounded_rect_sdf_gradient");
        format!("rounded_rect_sdf_gradient({pos}, {size}, 0.5 * min({size}.x, {size}.y))")
    }
}
impl BuildBindGroup for RoundedBar {
    fn update_layout(&self, _layout: &mut super::UniformLayout) {}

    fn write_uniform<B: encase::internal::BufferMut>(
        &self,
        _layout: &mut super::UniformLayout,
        _writer: &mut encase::internal::Writer<B>,
    ) {
    }
}
