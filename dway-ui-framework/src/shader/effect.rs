use bevy::render::render_resource::{
    encase::internal::{BufferMut, Writer},
    AsBindGroupError,
};

use super::{
    fill::{Fill, FillColor},
    shape::Shape,
    BuildBindGroup, Expr, ShaderBuilder, ShaderVariables,
};
use crate::prelude::*;

pub trait Effect: BuildBindGroup {
    fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables);
}

#[derive(Clone, Debug, Default, Interpolation)]
pub struct Shadow {
    pub color: LinearRgba,
    pub offset: Vec2,
    pub margin: Vec2,
    pub radius: f32,
}

impl Shadow {
    pub fn new(color: Color, offset: Vec2, margin: Vec2, radius: f32) -> Self {
        Self {
            color: color.to_linear(),
            offset,
            margin,
            radius,
        }
    }
}
impl Effect for Shadow {
    fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let ShaderVariables { pos, size } = var;
        builder.import_from_builtin("sigmoid");
        builder.import_from_builtin("mix_alpha");
        let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
        let uniform_offset = builder.get_uniform("offset", "", "vec2<f32>");
        let uniform_margin = builder.get_uniform("margin", "", "vec2<f32>");
        let uniform_radius = builder.get_uniform("radius", "", "f32");
        let (pos_var, pos_stat) =
            builder.add_var("shadow_pos", format!("{pos} - {uniform_offset}"));
        let (size_var, size_stat) =
            builder.add_var("shadow_size", format!("{size} + 2.0 * {uniform_margin}"));
        let shadow_d_expr = builder.in_namespace(shape_ns, |builder| {
            S::to_wgsl(
                builder,
                &ShaderVariables {
                    pos: pos_var.clone(),
                    size: size_var.clone(),
                },
            )
        });
        let vertex_code = format!("
                {{
                    let shadow_pos = {uniform_offset} + (vertex_uv - vec2(0.5)) * 4.0 * {uniform_margin};
                    let shadow_size = size + 2.0 * {uniform_margin};
                    out.position = view.clip_from_world * vec4<f32>(vertex_position + vec3(shadow_pos, 0.0), 1.0);
                    out.uv = vertex_uv + shadow_pos / size;
                }}
            "); // TODO 需要优化
        builder.vertex_inner += &*vertex_code;
        let fragment_code = format!("
                {{
                    {pos_stat}
                    {size_stat}
                    let shadow_d = {shadow_d_expr};
                    let shadow_alpha = 1.42 * (1.0 - sigmoid(shadow_d / {uniform_radius}));
                    if shadow_alpha > 1.0 / 16.0 {{
                        out = mix_alpha(out, vec4({uniform_color}.rgb, shadow_alpha * {uniform_color}.a));
                        if out.a > 255.0/256.0 {{
                            return out;
                        }}
                    }}
                }}
            ");
        builder.fragment_inner += &*fragment_code;
    }
}
impl BuildBindGroup for Shadow {
    fn bind_group_layout_entries(_builder: &mut super::BindGroupLayoutBuilder) {
    }

    fn unprepared_bind_group(
        &self,
        _builder: &mut super::BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        Ok(())
    }

    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.color);
        layout.update_layout(&self.offset);
        layout.update_layout(&self.margin);
        layout.update_layout(&self.radius);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.color, writer);
        layout.write_uniform(&self.offset, writer);
        layout.write_uniform(&self.margin, writer);
        layout.write_uniform(&self.radius, writer);
    }
}

#[derive(Clone, Debug, Default, Interpolation)]
pub struct InnerShadow<F: Fill = FillColor> {
    pub filler: F,
    pub color: LinearRgba,
    pub offset: Vec2,
    pub radius: f32,
}

impl<F: Fill> InnerShadow<F> {
    pub fn new(filler: impl Into<F>, color: Color, offset: Vec2, radius: f32) -> Self {
        Self {
            filler: filler.into(),
            color: color.to_linear(),
            offset,
            radius,
        }
    }
}
impl<F: Fill> Effect for InnerShadow<F> {
    fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let ShaderVariables { pos, size } = var;
        let color_expr = builder.in_new_namespace("filler", |builder| F::to_wgsl(builder, var));
        builder.import_from_builtin("sigmoid");
        builder.import_from_builtin("mix_alpha");
        builder.import_from_builtin("mix_color");
        let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
        let uniform_offset = builder.get_uniform("offset", "", "vec2<f32>");
        let uniform_radius = builder.get_uniform("radius", "", "f32");
        let (pos_var, pos_stat) =
            builder.add_var("shadow_pos", format!("{pos} - {uniform_offset}"));
        let (size_var, size_stat) = builder.add_var("shadow_size", size.to_string());
        let shadow_d_expr = builder.in_namespace(shape_ns, |builder| {
            S::to_wgsl(
                builder,
                &ShaderVariables {
                    pos: pos_var.clone(),
                    size: size_var.clone(),
                },
            )
        });
        let fragment_code = format!("
                {{
                    {pos_stat}
                    {size_stat}
                    if shape_d<0.5 {{
                        out = mix_alpha(out, mix_color({color_expr}, shape_d));
                    }}
                    if shape_d < 0.0 {{
                        let shadow_d = -{shadow_d_expr};
                        let shadow_alpha = 1.42 * (1.0 - sigmoid(shadow_d / {uniform_radius}));
                        if shadow_alpha > 1.0 / 16.0 {{
                            out = mix_alpha(out, vec4({uniform_color}.rgb, shadow_alpha * {uniform_color}.a));
                            if out.a > 255.0/256.0 {{
                                return out;
                            }}
                        }}
                    }}
                }}
            ");
        builder.fragment_inner += &*fragment_code;
    }
}
impl<F: Fill> BuildBindGroup for InnerShadow<F> {
    fn bind_group_layout_entries(_builder: &mut super::BindGroupLayoutBuilder) {
    }

    fn unprepared_bind_group(
        &self,
        _builder: &mut super::BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        Ok(())
    }

    fn update_layout(&self, layout: &mut super::UniformLayout) {
        self.filler.update_layout(layout);
        layout.update_layout(&self.color);
        layout.update_layout(&self.offset);
        layout.update_layout(&self.radius);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        self.filler.write_uniform(layout, writer);
        layout.write_uniform(&self.color, writer);
        layout.write_uniform(&self.offset, writer);
        layout.write_uniform(&self.radius, writer);
    }
}

#[derive(Clone, Debug, Default, Interpolation)]
pub struct Border<F: Fill = FillColor> {
    pub filler: F,
    pub width: f32,
}
impl<F: Fill> Border<F> {
    pub fn with_filler(filler: F, width: f32) -> Self {
        Self { filler, width }
    }
}
impl Border<FillColor> {
    pub fn new(color: Color, width: f32) -> Self {
        Self {
            filler: FillColor::new(color),
            width,
        }
    }
}
impl<F: Fill> Effect for Border<F> {
    fn to_wgsl<S: Shape>(_shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let color_expr = builder.in_new_namespace("filler", |builder| F::to_wgsl(builder, var));
        let uniform_width = builder.get_uniform("width", "", "f32");
        builder.import_from_builtin("mix_color");
        builder.import_from_builtin("mix_alpha");
        let code = format!(
                "
                {{
                    let border_d = abs(shape_d + (0.5 - 1.0/16.0) * {uniform_width}) - 0.5 * {uniform_width};
                    if border_d < 0.5 {{
                        out = mix_alpha(out, mix_color({color_expr}, border_d));
                        if out.a > 255.0/256.0 {{
                            return out;
                        }}
                    }}
                }}
            "
            );
        builder.fragment_inner += &*code;
    }
}
impl<F: Fill> BuildBindGroup for Border<F> {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        self.filler.update_layout(layout);
        layout.update_layout(&self.width);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        self.filler.write_uniform(layout, writer);
        layout.write_uniform(&self.width, writer);
    }
}

#[derive(Clone, Debug, Default, Interpolation)]
pub struct Arc {
    pub angle: [f32; 2],
    pub width: f32,
}

impl Arc {
    pub fn new(angle: [f32; 2], width: f32) -> Self {
        Self { angle, width }
    }
}
impl Shape for Arc {
    fn register_uniforms(_builder: &mut ShaderBuilder) {
    }

    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let uniform_angle = builder.get_uniform("angle", "", "vec2<f32>");
        let uniform_width = builder.get_uniform("width", "", "f32");
        builder.import_from_builtin("arc_sdf");
        format!("arc_sdf({pos}, {uniform_angle}, {uniform_width}, 0.5*min({size}.x,{size}.y))")
    }

    fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
        let ShaderVariables { pos, size } = var;
        let uniform_angle = builder.get_uniform("angle", "", "vec2<f32>");
        let uniform_width = builder.get_uniform("width", "", "f32");
        builder.import_from_builtin("arc_sdf_gradient");
        format!(
            "arc_sdf_gradient({pos}, {uniform_angle}, {uniform_width}, 0.5*min({size}.x,{size}.y))"
        )
    }
}
impl BuildBindGroup for Arc {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.angle);
        layout.update_layout(&self.width);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.angle, writer);
        layout.write_uniform(&self.width, writer);
    }
}

impl<T: Fill> Effect for T {
    fn to_wgsl<S: Shape>(_shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let expr_color = T::to_wgsl(builder, var);
        builder.import_from_builtin("mix_color");
        builder.import_from_builtin("mix_alpha");
        let code = format!(
            "
                if shape_d<0.5 {{
                    out = mix_alpha(out, mix_color({expr_color}, shape_d));
                    if out.a > 255.0/256.0 {{
                        return out;
                    }}
                }}
            "
        );
        builder.fragment_inner += &*code;
    }
}

#[derive(Debug, Clone, Default, Interpolation)]
pub struct Fake3D {
    pub color: LinearRgba,
    pub half_dir: Vec3,
    pub corner: f32,
}

impl Fake3D {
    pub fn new(color: Color, light_direction: Vec3, corner: f32) -> Self {
        Self {
            color: color.to_linear(),
            half_dir: (light_direction + Vec3::Z).normalize(),
            corner,
        }
    }
}
impl Effect for Fake3D {
    fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let normal_expr =
            builder.in_namespace(shape_ns, |builder| S::to_gradient_wgsl(builder, var));
        let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
        let uniform_half_dir = builder.get_uniform("half_dir", "", "vec3<f32>");
        let uniform_corner = builder.get_uniform("corner", "", "f32");
        let code = format!("
                {{
                    if -{uniform_corner} < shape_d && shape_d < 0 {{
                        let normal2d = {normal_expr};
                        let fixed_x = shape_d + {uniform_corner};
                        let border_normal2d = normalize(vec2(fixed_x, sqrt({uniform_corner} * {uniform_corner} - fixed_x * fixed_x )));
                        let normal3d = vec3(normal2d.x * border_normal2d.x, normal2d.y * border_normal2d.x, border_normal2d.y);
                        let color = vec4( saturate(dot(normal3d, {uniform_half_dir} )) * {uniform_color}.rgb, {uniform_color}.a );

                        out = mix_alpha(out, color);
                        if out.a > 255.0/256.0 {{
                            return out;
                        }}
                    }}
                }}
            ");
        builder.fragment_inner += &*code;
    }
}
impl BuildBindGroup for Fake3D {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.color);
        layout.update_layout(&self.half_dir);
        layout.update_layout(&self.corner);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.color, writer);
        layout.write_uniform(&self.half_dir, writer);
        layout.write_uniform(&self.corner, writer);
    }
}

macro_rules! impl_effect_for_tuple {
        () => { };
        ($first_elem:ident,$($elem:ident,)*) => {
            #[allow(non_snake_case)]
            impl<$first_elem: Effect,$($elem: Effect),* > Effect for ($first_elem,$($elem),*){
                fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
                    builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::to_wgsl::<S>(shape_ns, builder, var));
                    $( builder.in_new_namespace(stringify!($elem), |builder|$elem::to_wgsl::<S>(shape_ns, builder, var)); )*
                }
            }
            impl_effect_for_tuple!($($elem,)*);
        };
    }
impl_effect_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);
