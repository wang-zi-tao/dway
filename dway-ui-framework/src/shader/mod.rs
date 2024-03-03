use crate::prelude::*;
use bevy::render::{render_asset::RenderAsset, render_resource::encase::private::Metadata};
use bevy::{
    asset::{embedded_asset, io::embedded::EmbeddedAssetRegistry, load_internal_asset},
    render::{
        render_asset::RenderAssets,
        render_resource::{
            encase::{internal::WriteInto, UniformBuffer},
            AsBindGroup, AsBindGroupError, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferInitDescriptor, BufferUsages,
            OwnedBindingResource, RenderPipelineDescriptor, SamplerBindingType, ShaderRef,
            ShaderStages, ShaderType, TextureSampleType, TextureViewDimension, UnpreparedBindGroup,
        },
        renderer::RenderDevice,
        texture::FallbackImage,
    },
};
use dway_ui_derive::Interpolation;
use encase::{
    internal::{AlignmentValue, BufferMut, SizeValue, Writer},
    DynamicUniformBuffer,
};
use std::{
    any::{type_name, TypeId},
    collections::BTreeSet,
    hash::Hash,
    marker::PhantomData,
    mem::size_of,
    path::PathBuf,
};

use self::{effect::Effect, shape::Shape, transform::Transform};

type Ident = String;
type Expr = String;
type Stat = String;

#[derive(Clone)]
pub struct ShaderVariables {
    pub pos: Ident,
    pub size: Ident,
}

#[derive(Default)]
pub struct ShaderBuilder {
    imports: BTreeSet<String>,
    fragment_inner: String,
    vertex_inner: String,
    uniforms: Vec<(String, String, String)>,
    binding: Vec<(String, String, String)>,
    vertex_fields: Vec<(String, String, String)>,
    prefixes: Vec<String>,
}

impl ShaderBuilder {
    pub fn get_uniform(&mut self, name: &str, attr: &str, ty: &str) -> Ident {
        let name = format!(
            "{}_{}",
            self.prefixes.last().map(|s| &**s).unwrap_or(""),
            name
        );
        if self.uniforms.iter().find(|(_, k, _)| k == &name).is_none() {
            self.uniforms
                .push((attr.to_string(), name.clone(), ty.to_string()));
        }
        format!("uniforms.{name}")
    }
    pub fn get_binding(&mut self, name: &str, attr: &str, ty: &str) -> Ident {
        let name = format!(
            "{}_{}",
            self.prefixes.last().map(|s| &**s).unwrap_or(""),
            name
        );
        if self.binding.iter().find(|(_, k, _)| k == &name).is_none() {
            self.binding
                .push((attr.to_string(), name.clone(), ty.to_string()));
        }
        name
    }
    pub fn add_import(&mut self, import: &str) {
        self.imports.insert(import.to_string());
    }
    pub fn import_from_builtin(&mut self, import: &str) {
        self.imports
            .insert(format!("dway_ui_framework::shader::framework::{import}"));
    }
    pub fn add_var(&mut self, name: &str, value: Expr) -> (Ident, Stat) {
        let name = if let Some(prefix) = self.prefixes.last() {
            format!("{prefix}_{name}")
        } else {
            name.to_string()
        };
        let stat = format!(
            "
            let {name} = {value};
        "
        );
        (name, stat)
    }
    pub fn in_namespace<R>(&mut self, namespace: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        self.prefixes.push(namespace.to_string());
        let r = f(self);
        self.prefixes.pop();
        r
    }
    pub fn in_new_namespace<R>(&mut self, namespace: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let full_namespace = self.new_namespace(namespace);
        self.in_namespace(&full_namespace, f)
    }
    pub fn new_namespace(&self, ns: &str) -> String {
        if let Some(prefix) = self.prefixes.last() {
            format!("{prefix}_{ns}")
        } else {
            ns.to_string()
        }
    }
    pub fn build(&self) -> String {
        let Self {
            fragment_inner,
            vertex_inner,
            ..
        } = self;
        let usees = self
            .imports
            .iter()
            .map(|u| format!("#import {u}"))
            .collect::<Vec<_>>()
            .join(&"\n");
        let uniforms = self
            .uniforms
            .iter()
            .enumerate()
            .map(|(i, (a, k, t))| format!("@location({i}) {a} {k}: {t},"))
            .collect::<Vec<_>>()
            .join(&"\n");
        let bindings = self
            .binding
            .iter()
            .enumerate()
            .map(|(i, (attr, name, ty))| {
                format!("@group(1) @binding({}) {attr} var {name}: {ty};", i + 1)
            })
            .collect::<Vec<_>>()
            .join(&"\n");
        let vertex_fields = self
            .vertex_fields
            .iter()
            .map(|(p, k, t)| format!("{p} {k}: {t},"))
            .collect::<Vec<_>>()
            .join(&"\n");
        format!(
            "
#import bevy_render::view::View
#import dway_ui_framework::shader::framework::sdf_visualition
{usees}

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var<uniform> uniforms: Settings;
struct Settings {{
{uniforms}
}}
{bindings}

struct VertexOutput {{
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
{vertex_fields}
}};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) border_widths: vec4<f32>,
) -> VertexOutput {{
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.border_widths = border_widths;
    var rect_position = (vertex_uv - 0.5) * size;
    var rect_size = size;
    var extend_left = 0.0;
    var extend_right = 0.0;
    var extend_top = 0.0;
    var extend_bottom = 0.0;
    out.uv = vertex_uv;
    out.size = size;
{vertex_inner}
    return out;
}}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    var out = vec4(1.0, 1.0, 1.0, 0.0);
    let rect_position = (in.uv - 0.5) * in.size;
    let rect_size = in.size;
{fragment_inner}
    return out;
}}
"
        )
    }
}

pub struct BindGroupBuilder<'l> {
    pub output: Vec<(u32, OwnedBindingResource)>,
    pub layout: &'l BindGroupLayout,
    pub render_device: &'l RenderDevice,
    pub images: &'l RenderAssets<Image>,
    pub fallback_image: &'l FallbackImage,
}
impl<'l> BindGroupBuilder<'l> {
    pub fn new(
        layout: &'l BindGroupLayout,
        render_device: &'l RenderDevice,
        images: &'l RenderAssets<Image>,
        fallback_image: &'l FallbackImage,
    ) -> Self {
        Self {
            output: vec![],
            layout,
            render_device,
            images,
            fallback_image,
        }
    }

    pub fn binding_number(&self) -> u32 {
        self.output.len() as u32
    }

    pub fn add_uniform_buffer<R: WriteInto + ShaderType>(
        &mut self,
        render: &R,
    ) -> Result<(), AsBindGroupError> {
        let mut buffer = UniformBuffer::new(vec![]);
        buffer
            .write(render)
            .map_err(|_| AsBindGroupError::RetryNextUpdate)?;
        self.output.push((
            0,
            OwnedBindingResource::Buffer(self.render_device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: None,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    contents: buffer.as_ref(),
                },
            )),
        ));
        Ok(())
    }

    pub fn build(self) -> UnpreparedBindGroup<()> {
        UnpreparedBindGroup {
            bindings: self.output,
            data: (),
        }
    }

    pub fn add_image(&mut self, image: &Handle<Image>) -> Result<(), AsBindGroupError> {
        let image = self
            .images
            .get(image)
            .ok_or_else(|| AsBindGroupError::RetryNextUpdate)?;
        self.output.push((
            self.binding_number(),
            OwnedBindingResource::TextureView(image.texture_view.clone()),
        ));
        self.output.push((
            self.binding_number(),
            OwnedBindingResource::Sampler(image.sampler.clone()),
        ));
        Ok(())
    }
}

pub struct BindGroupLayoutBuilder<'l> {
    pub output: Vec<BindGroupLayoutEntry>,
    pub size: usize,
    pub render_device: &'l RenderDevice,
}
impl<'l> BindGroupLayoutBuilder<'l> {
    pub fn new(render_device: &'l RenderDevice) -> Self {
        Self {
            render_device,
            output: Default::default(),
            size: 0,
        }
    }

    pub fn binding_number(&self) -> u32 {
        self.output.len() as u32 + 1
    }

    pub fn build(mut self) -> Vec<BindGroupLayoutEntry> {
        self.output.insert(
            0,
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::all(),
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        );
        self.output
    }

    pub fn add_image(&mut self) {
        self.output.push(BindGroupLayoutEntry {
            binding: self.binding_number(),
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
        self.output.push(BindGroupLayoutEntry {
            binding: self.binding_number(),
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        });
    }
}

pub struct UniformLayout {
    pub alignment: AlignmentValue,
    pub size: u64,
}

impl UniformLayout {
    fn update_layout<T: ShaderType>(&mut self, value: &T) {
        let filed_layout = T::METADATA;
        let round_up_size = if self.size == 0 {
            0
        } else {
            filed_layout
                .alignment
                .round_up_size(SizeValue::new(self.size))
                .get()
        };
        self.alignment = AlignmentValue::max([self.alignment, filed_layout.alignment]);
        self.size = round_up_size;
        self.size += value.size().get();
    }
    fn write_uniform<T: ShaderType + WriteInto, B: BufferMut>(
        &mut self,
        value: &T,
        writer: &mut Writer<B>,
    ) {
        let filed_layout = T::METADATA;
        let round_up_size = if self.size == 0 {
            0
        } else {
            filed_layout
                .alignment
                .round_up_size(SizeValue::new(self.size))
                .get()
        };
        self.alignment = AlignmentValue::max([self.alignment, filed_layout.alignment]);
        writer.advance((round_up_size - self.size) as usize);
        self.size = round_up_size;
        value.write_into(writer);
        self.size += value.size().get();
    }
}

impl Default for UniformLayout {
    fn default() -> Self {
        Self {
            alignment: AlignmentValue::new(32),
            size: Default::default(),
        }
    }
}

pub trait BuildBindGroup: Clone + Send + Sync + 'static {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {}
    fn unprepared_bind_group(
        &self,
        _builder: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        Ok(())
    }
    fn update_layout(&self, layout: &mut UniformLayout);
    fn write_uniform<B: BufferMut>(&self, layout: &mut UniformLayout, writer: &mut Writer<B>);
}

macro_rules! impl_build_bind_group_for_tuple {
    () => {};
    ($first_elem:ident,$($elem:ident,)*) => {
        #[allow(non_snake_case)]
        impl<$first_elem: BuildBindGroup,$($elem: BuildBindGroup),* > BuildBindGroup for ($first_elem,$($elem),*){
            fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
                $first_elem::bind_group_layout_entries(builder);
                $($elem::bind_group_layout_entries(builder);)*
            }

            fn unprepared_bind_group(&self, builder: &mut BindGroupBuilder)
                -> Result<(), bevy::render::render_resource::AsBindGroupError> {
                    let ($first_elem,$($elem),*) = self;
                    $first_elem.unprepared_bind_group(builder)?;
                    $($elem.unprepared_bind_group(builder)?;)*
                    Ok(())
            }
            fn update_layout(&self, layout: &mut UniformLayout) {
                let ($first_elem,$($elem),*) = self;
                $first_elem.update_layout(layout);
                $($elem.update_layout(layout);)*
            }
            fn write_uniform<B: BufferMut>(&self, layout: &mut UniformLayout, writer: &mut Writer<B>) {
                let ($first_elem,$($elem),*) = self;
                $first_elem.write_uniform(layout, writer);
                $($elem.write_uniform(layout, writer);)*
            }
        }
        impl_build_bind_group_for_tuple!($($elem,)*);
    };
}
impl_build_bind_group_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);

macro_rules! impl_interpolation_for_tuple {
    () => {};
    ($($elem_field:tt : $elem:ident,)*) => {
        #[allow(non_snake_case)]
        impl<$($elem: Interpolation),* > Interpolation for ($($elem,)*){
            fn interpolation(&self, other: &Self, v: f32) -> Self {
                (
                    $(Interpolation::interpolation(&self.$elem_field, &other.$elem_field, v),)*
                )
            }
        }
    };
}
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,3:E3,4:E4,5:E5,6:E6,7:E7,);
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,3:E3,4:E4,5:E5,6:E6,);
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,3:E3,4:E4,5:E5,);
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,3:E3,4:E4,);
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,3:E3,);
impl_interpolation_for_tuple!(0:E0,1:E1,2:E2,);
impl_interpolation_for_tuple!(0:E0,1:E1,);
impl_interpolation_for_tuple!(0:E0,);

pub mod effect {
    use bevy::render::render_resource::encase::internal::WriteInto;
    use encase::{
        internal::{BufferMut, Writer},
        ShaderType,
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
        pub color: Color,
        pub offset: Vec2,
        pub margin: Vec2,
        pub radius: f32,
    }

    impl Shadow {
        pub fn new(color: Color, offset: Vec2, margin: Vec2, radius: f32) -> Self {
            Self {
                color,
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
                    out.position = view.view_proj * vec4<f32>(vertex_position + vec3(shadow_pos, 0.0), 1.0);
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
        fn bind_group_layout_entries(_builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
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
        pub color: Color,
        pub offset: Vec2,
        pub radius: f32,
    }

    impl<F: Fill> InnerShadow<F> {
        pub fn new(filler: impl Into<F>, color: Color, offset: Vec2, radius: f32) -> Self {
            Self {
                filler: filler.into(),
                color,
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
            let (size_var, size_stat) = builder.add_var("shadow_size", format!("{size}"));
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
        fn bind_group_layout_entries(_builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
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
        fn register_uniforms(builder: &mut ShaderBuilder) {}

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
            format!("arc_sdf_gradient({pos}, {uniform_angle}, {uniform_width}, 0.5*min({size}.x,{size}.y))")
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
        pub color: Color,
        pub half_dir: Vec3,
        pub corner: f32,
    }

    impl Fake3D {
        pub fn new(color: Color, light_direction: Vec3, corner: f32) -> Self {
            Self {
                color,
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
}

pub mod fill {
    use bevy::render::render_resource::AsBindGroupError;

    use super::{BuildBindGroup, Expr, ShaderBuilder};
    use crate::{prelude::*, shader::ShaderVariables};

    pub trait Fill: BuildBindGroup {
        fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr;
    }

    #[derive(Clone, Default, Debug, Interpolation)]
    pub struct Gradient {
        pub color: Color,
        pub delta_color: Vec4,
        pub direction: Vec2,
    }

    impl Gradient {
        pub fn new(color: Color, delta_color: Vec4, direction: Vec2) -> Self {
            Self {
                color,
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

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
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
        fn update_layout(&self, _layout: &mut super::UniformLayout) {}
        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            _layout: &mut super::UniformLayout,
            _writer: &mut encase::internal::Writer<B>,
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
        pub color: Color,
    }

    impl From<Color> for FillColor {
        fn from(value: Color) -> Self {
            Self { color: value }
        }
    }

    impl FillColor {
        pub fn new(color: Color) -> Self {
            Self { color }
        }
    }
    impl Fill for FillColor {
        fn to_wgsl(builder: &mut ShaderBuilder, _pos: &ShaderVariables) -> Expr {
            let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
            uniform_color
        }
    }
    impl BuildBindGroup for FillColor {
        fn update_layout(&self, layout: &mut super::UniformLayout) {
            layout.update_layout(&self.color);
        }

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
            layout.write_uniform(&self.color, writer);
        }
    }

    #[derive(Clone, Default, Debug)]
    pub struct FillImage {
        pub min_uv: Vec2,
        pub size_uv: Vec2,
        pub image: Handle<Image>,
    }
    impl From<Handle<Image>> for FillImage {
        fn from(value: Handle<Image>) -> Self {
            Self {
                min_uv: Vec2::ZERO,
                size_uv: Vec2::ONE,
                image: value,
            }
        }
    }
    impl FillImage {
        pub fn new(min_uv: Vec2, size_uv: Vec2, image: Handle<Image>) -> Self {
            Self {
                min_uv,
                size_uv,
                image,
            }
        }
    }
    impl Fill for FillImage {
        fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
            let ShaderVariables { pos, .. } = var;
            let uniform_min_uv = builder.get_uniform("min_uv", "", "vec2<f32>");
            let uniform_size_uv = builder.get_uniform("size_uv", "", "vec2<f32>");
            let var_image_texture = builder.get_binding("image_texture", "", "texture_2d<f32>");
            let var_image_sampler = builder.get_binding("image_sampler", "", "sampler");
            format!("textureSample({var_image_texture}, {var_image_sampler}, {pos} * {uniform_size_uv} + {uniform_min_uv})")
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
            layout.update_layout(&self.min_uv);
            layout.update_layout(&self.size_uv);
        }

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
            layout.write_uniform(&self.min_uv, writer);
            layout.write_uniform(&self.size_uv, writer);
        }
    }
}

pub mod shape {
    use super::{
        effect::Effect, BuildBindGroup, Expr, ShaderBuilder, ShaderVariables, ShapeRender,
    };
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

        fn register_uniforms(builder: &mut ShaderBuilder) {}

        fn to_gradient_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
            let ShaderVariables { pos, size } = var;
            builder.import_from_builtin("circle_sdf_gradient");
            format!("circle_sdf_gradient({pos}, 0.5 * min({size}.x, {size}.y))")
        }
    }
    impl BuildBindGroup for Circle {
        fn update_layout(&self, layout: &mut super::UniformLayout) {}

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
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
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
        }
    }
}

pub mod transform {
    use crate::prelude::*;

    use super::{BuildBindGroup, Expr, Material, ShaderBuilder, ShaderVariables, Transformed};
    pub trait Transform: BuildBindGroup {
        fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables;

        fn then<R: Material>(self, material: R) -> Transformed<R, Self> {
            Transformed::new(material, self)
        }
    }

    #[derive(Clone, Default, Debug, Interpolation)]
    pub struct Translation {
        pub offset: Vec2,
    }

    impl Translation {
        pub fn new(offset: Vec2) -> Self {
            Self { offset }
        }
    }
    impl Transform for Translation {
        fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
            let ShaderVariables { pos, .. } = var;
            let uniform_offset = builder.get_uniform("offset", "", "vec2<f32>");
            ShaderVariables {
                pos: format!("({pos}-{uniform_offset})"),
                ..var.clone()
            }
        }
    }
    impl BuildBindGroup for Translation {
        fn update_layout(&self, layout: &mut super::UniformLayout) {
            layout.update_layout(&self.offset);
        }

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
            layout.write_uniform(&self.offset, writer);
        }
    }

    #[derive(Clone, Default, Debug, Interpolation)]
    pub struct Rotation {
        pub rotation: f32,
    }

    impl Rotation {
        pub fn new(rotation: f32) -> Self {
            Self { rotation }
        }
    }
    impl Transform for Rotation {
        fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
            let ShaderVariables { pos, .. } = var;
            builder.import_from_builtin("sdf_rotation");
            let uniform_rotation = builder.get_uniform("rotation", "", "f32");
            ShaderVariables {
                pos: format!("sdf_rotation({pos}, {uniform_rotation})"),
                ..var.clone()
            }
        }
    }
    impl BuildBindGroup for Rotation {
        fn update_layout(&self, layout: &mut super::UniformLayout) {
            layout.update_layout(&self.rotation);
        }

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
            layout.write_uniform(&self.rotation, writer);
        }
    }

    #[derive(Clone, Default, Debug, Interpolation)]
    pub struct Margins {
        pub margins: Vec4,
    }
    impl From<Vec4> for Margins {
        fn from(value: Vec4) -> Self {
            Self { margins: value }
        }
    }
    impl Margins {
        pub fn all(value: f32) -> Self {
            Self {
                margins: Vec4::splat(value),
            }
        }
        pub fn axes(horizontal: f32, vertical: f32) -> Self {
            Self {
                margins: Vec4::new(horizontal, horizontal, vertical, vertical),
            }
        }
        pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
            Self {
                margins: Vec4::new(left, right, top, bottom),
            }
        }
    }
    impl Transform for Margins {
        fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
            let ShaderVariables { size, pos } = var;
            let uniform_margins = builder.get_uniform("margins", "", "vec4<f32>");
            ShaderVariables {
                size: format!("( {size} - vec2({uniform_margins}.x+{uniform_margins}.y,{uniform_margins}.z+{uniform_margins}.w) )"),
                pos: format!("( {pos} - 0.5 * vec2({uniform_margins}.x-{uniform_margins}.y,{uniform_margins}.z-{uniform_margins}.w) )"),
            }
        }
    }
    impl BuildBindGroup for Margins {
        fn update_layout(&self, layout: &mut super::UniformLayout) {
            layout.update_layout(&self.margins);
        }

        fn write_uniform<B: encase::internal::BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut encase::internal::Writer<B>,
        ) {
            layout.write_uniform(&self.margins, writer);
        }
    }

    macro_rules! impl_transform_for_tuple {
        () => { };
        ($first_elem:ident,$($elem:ident,)*) => {
            impl<$first_elem: Transform,$($elem: Transform),* > Transform for ($first_elem,$($elem),*){
                #[allow(non_snake_case)]
                fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
                    let mut var = var.clone();
                    var = builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::transform(builder, &var).clone());
                    $( var = builder.in_new_namespace(stringify!($elem), |builder|$elem::transform(builder, &var).clone()); )*
                    var
                }
            }
            impl_transform_for_tuple!($($elem,)*);
        };
    }
    impl_transform_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);
}

pub trait Material: BuildBindGroup {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables);

    fn into_asset(self) -> ShaderAsset<Self> {
        ShaderAsset::new(self)
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct ShapeRender<S: Shape, E: Effect> {
    pub shape: S,
    pub effect: E,
}

impl<S: Shape, E: Effect> ShapeRender<S, E> {
    pub fn new(shape: impl Into<S>, effect: impl Into<E>) -> Self {
        Self {
            shape: shape.into(),
            effect: effect.into(),
        }
    }

    pub fn with_transform<T: Transform>(self, transform: T) -> Transformed<Self, T> {
        Transformed {
            render: self,
            transform,
        }
    }
}

impl<S: Shape, E: Effect> Material for ShapeRender<S, E> {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let ns = builder.new_namespace("shape");
        let shape_vars = ShaderVariables {
            pos: format!("{ns}_pos"),
            size: format!("{ns}_size"),
        };
        builder.fragment_inner.push_str(
            &"
        {
        ",
        );
        builder.in_namespace(&ns, |builder| {
            let ShaderVariables { pos, size } = &var;
            let ShaderVariables {
                pos: shape_pos,
                size: shape_size,
            } = &shape_vars;
            let expr_d = S::to_wgsl(builder, &shape_vars);
            let code = format!(
                "
                let {shape_pos} = {pos};
                let {shape_size} = {size};
                let shape_d = {expr_d};
            "
            );
            builder.fragment_inner += &*code;
            S::register_uniforms(builder)
        });
        builder.in_new_namespace("effect", |builder| {
            E::to_wgsl::<S>(&ns, builder, &shape_vars)
        });
        builder.fragment_inner.push_str(
            &"
        }
        ",
        );
    }
}
impl<S: Shape, E: Effect> BuildBindGroup for ShapeRender<S, E> {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
        S::bind_group_layout_entries(builder);
        E::bind_group_layout_entries(builder);
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        self.shape.unprepared_bind_group(builder)?;
        self.effect.unprepared_bind_group(builder)?;
        Ok(())
    }

    fn update_layout(&self, layout: &mut UniformLayout) {
        self.shape.update_layout(layout);
        self.effect.update_layout(layout);
    }

    fn write_uniform<B: BufferMut>(&self, layout: &mut UniformLayout, writer: &mut Writer<B>) {
        self.shape.write_uniform(layout, writer);
        self.effect.write_uniform(layout, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Transformed<S: Material, T: Transform> {
    pub render: S,
    pub transform: T,
}

impl<S: Material, T: Transform> Transformed<S, T> {
    pub fn new(render: impl Into<S>, transform: impl Into<T>) -> Self {
        Self {
            render: render.into(),
            transform: transform.into(),
        }
    }
}
impl<S: Material, T: Transform> BuildBindGroup for Transformed<S, T> {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
        T::bind_group_layout_entries(builder);
        S::bind_group_layout_entries(builder);
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        self.transform.unprepared_bind_group(builder)?;
        self.render.unprepared_bind_group(builder)?;
        Ok(())
    }

    fn update_layout(&self, layout: &mut UniformLayout) {
        self.transform.update_layout(layout);
        self.render.update_layout(layout);
    }

    fn write_uniform<B: BufferMut>(&self, layout: &mut UniformLayout, writer: &mut Writer<B>) {
        self.transform.write_uniform(layout, writer);
        self.render.write_uniform(layout, writer);
    }
}
impl<S: Material, T: Transform> Material for Transformed<S, T> {
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) {
        let ns = builder.new_namespace("transform");
        let ShaderVariables { pos, size } =
            builder.in_namespace("transform", |builder| T::transform(builder, var));
        let transformed_pos_ident = format!("{ns}_pos");
        let transformed_size_ident = format!("{ns}_size");
        let code = format!(
            "
            let {transformed_pos_ident} = {pos};
            let {transformed_size_ident} = {size};
        "
        );
        builder.vertex_inner += &*code;
        builder.fragment_inner += &*code;
        S::to_wgsl(
            builder,
            &ShaderVariables {
                pos: transformed_pos_ident,
                size: transformed_size_ident,
            },
        );
    }
}

macro_rules! impl_render_for_tuple {
    () => { };
    ($first_elem:ident,$($elem:ident,)*) => {
        impl<$first_elem: Material,$($elem: Material),* > Material for ($first_elem,$($elem),*){
            #[allow(non_snake_case)]
            fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) {
                builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::to_wgsl(builder, var));
                $( builder.in_new_namespace(stringify!($elem), |builder|$elem::to_wgsl(builder, var)); )*
            }
        }
        impl_render_for_tuple!($($elem,)*);
    };
}
impl_render_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);

#[derive(Asset, Default, Debug, Interpolation)]
pub struct ShaderAsset<T: Material> {
    pub render: T,
}

impl<T: Material> From<T> for ShaderAsset<T> {
    fn from(value: T) -> Self {
        Self { render: value }
    }
}

impl<T: Material> ShaderAsset<T> {
    pub fn new(render: T) -> Self {
        Self { render }
    }

    pub fn to_wgsl() -> String {
        let mut builder = ShaderBuilder::default();
        let vars = ShaderVariables {
            pos: format!("rect_position"),
            size: format!("rect_size"),
        };
        T::to_wgsl(&mut builder, &vars);
        builder.build()
    }
    fn id() -> String {
        (format!("{:?}", TypeId::of::<T>())).replace(|c: char| c == ':', "=")
    }
    pub fn raw_path() -> String {
        format!("dway_ui_framework/render/gen/{}/render.wgsl", Self::id())
    }
    pub fn path() -> String {
        format!(
            "embedded://dway_ui_framework/render/gen/{}/render.wgsl",
            Self::id()
        )
    }

    pub fn plugin() -> ShaderPlugin<T> {
        ShaderPlugin::<T>::default()
    }
}

impl<T: Material> Clone for ShaderAsset<T> {
    fn clone(&self) -> Self {
        Self {
            render: self.render.clone(),
        }
    }
}

impl<T: Material> TypePath for ShaderAsset<T> {
    fn type_path() -> &'static str {
        type_name::<Self>()
    }

    fn short_type_path() -> &'static str {
        type_name::<Self>()
    }
}

impl<T: Material> WriteInto for ShaderAsset<T> {
    fn write_into<B>(&self, writer: &mut Writer<B>)
    where
        B: BufferMut,
    {
        let mut layout = UniformLayout::default();
        self.render.write_uniform(&mut layout, writer)
    }
}

impl<T: Material> ShaderType for ShaderAsset<T> {
    type ExtraMetadata = ();
    const METADATA: Metadata<()> = Metadata {
        alignment: AlignmentValue::new(1),
        has_uniform_min_alignment: false,
        min_size: SizeValue::new(32),
        extra: (),
    };
    fn min_size() -> std::num::NonZeroU64 {
        Self::METADATA.min_size().0
    }

    fn size(&self) -> std::num::NonZeroU64 {
        let mut layout = UniformLayout::default();
        self.render.update_layout(&mut layout);
        let size = layout.alignment.round_up_size(SizeValue::from(
            (layout.size as u64)
                .try_into()
                .unwrap_or(1.try_into().unwrap()),
        ));
        size.0
    }

    fn assert_uniform_compat() {
        Self::UNIFORM_COMPAT_ASSERT()
    }
}

impl<T: Material> AsBindGroup for ShaderAsset<T> {
    type Data = ();

    fn label() -> Option<&'static str> {
        Some(type_name::<Self>())
    }

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> std::prelude::v1::Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        let mut builder = BindGroupBuilder::new(layout, render_device, images, fallback_image);
        builder.add_uniform_buffer(self)?;
        BuildBindGroup::unprepared_bind_group(&self.render, &mut builder)?;
        Ok(builder.build())
    }

    fn bind_group_layout_entries(render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        let mut builder = BindGroupLayoutBuilder::new(render_device);
        <T as BuildBindGroup>::bind_group_layout_entries(&mut builder);
        builder.build()
    }
}

impl<T: Material> UiMaterial for ShaderAsset<T> {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn specialize(_descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {}
}

pub struct ShaderPlugin<T: Material>(PhantomData<T>);

impl<T: Material> ShaderPlugin<T> {
    pub fn add_inoto(self, app: &mut App) {
        if !app.is_plugin_added::<Self>() {
            app.add_plugins(self);
        }
    }
}

impl<T: Material> Default for ShaderPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Material> Plugin for ShaderPlugin<T> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<UiMaterialPlugin<ShaderAsset<T>>>() {
            let embedded = app.world.resource_mut::<EmbeddedAssetRegistry>();
            let path: PathBuf = ShaderAsset::<T>::raw_path().into();
            let wgsl = ShaderAsset::<T>::to_wgsl();
            trace!("add shader: {path:?}\n{wgsl}");
            embedded.insert_asset(std::path::PathBuf::new(), &path, wgsl.into_bytes());
            app.add_plugins(UiMaterialPlugin::<ShaderAsset<T>>::default());
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}

const FRAMEWORK_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(15628284168829255748903736059973599232);

pub struct ShaderFrameworkPlugin;
impl Plugin for ShaderFrameworkPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, FRAMEWORK_HANDLE, "framework.wgsl", Shader::from_wgsl);
    }
}

#[cfg(test)]
pub mod test {
    use anyhow::anyhow;
    use bevy::{
        app::{AppExit, PluginsState, ScheduleRunnerPlugin},
        core::FrameCount,
        core_pipeline::CorePipelinePlugin,
        input::{InputPlugin, InputSystem},
        render::{
            camera::RenderTarget,
            render_resource::{
                Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
            },
            view::screenshot::ScreenshotManager,
            RenderPlugin,
        },
        sprite::SpritePlugin,
        ui::UiPlugin,
        window::{PresentMode, PrimaryWindow},
        winit::WinitPlugin,
    };
    use bevy_image_export::{
        ImageExportBundle, ImageExportPlugin, ImageExportSettings, ImageExportSource,
    };
    use failure::format_err;
    use image::{DynamicImage, GenericImageView, Pixel};
    use lazy_static::lazy_static;
    use pretty_assertions::{assert_eq, assert_ne};
    use regex::Regex;
    use std::{
        borrow::Cow,
        path::Path,
        sync::{atomic::AtomicBool, Arc},
    };

    use crate::{
        tests::{assert_image_eq, compare_image, run_test_plugins, UnitTestPlugin},
        UiFrameworkPlugin,
    };

    use self::{
        effect::{Border, Shadow},
        fill::{FillColor, FillImage, Gradient},
        shape::{Circle, Rect, RoundedRect},
        transform::{Transform, Translation},
    };

    use super::*;

    lazy_static! {
        static ref RE: Regex = Regex::new(r"  +").unwrap();
    }

    fn simplify_wgsl<'l>(input: &'l str) -> String {
        let input = input.replace(|c: char| c.is_whitespace(), " ");
        RE.replace_all(&*input, " ").to_string()
    }

    fn test_render_type<R: Material>(except_path: &str, except_wgsl: &str) {
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
        test_render_type::<ShapeRender<RoundedRect, FillColor>>("embedded://dway_ui_framework/render/gen/TypeId { t= 188717855749609276234625418564726538671 }/render.wgsl",
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
@vertex fn vertex( @location(0) vertex_position: vec3<f32>, @location(1) vertex_uv: vec2<f32>, @location(2) size: vec2<f32>, @location(3) border_widths: vec4<f32>, ) -> VertexOutput { var out: VertexOutput; out.position = view.view_proj * vec4<f32>(vertex_position, 1.0); out.border_widths = border_widths; var rect_position = (vertex_uv - 0.5) * size; var rect_size = size; var extend_left = 0.0; var extend_right = 0.0; var extend_top = 0.0; var extend_bottom = 0.0; out.uv = vertex_uv; out.size = size; return out; } 
@fragment fn fragment(in: VertexOutput) -> @location(0) vec4<f32> { var out = vec4(1.0, 1.0, 1.0, 0.0); let rect_position = (in.uv - 0.5) * in.size; let rect_size = in.size; { let shape_pos = rect_position; let shape_size = rect_size; let shape_d = rounded_rect_sdf(shape_pos, shape_size, uniforms.shape_radius); if shape_d<0.5 { out = mix_alpha(out, mix_color(uniforms.effect_color, shape_d)); if out.a > 255.0/256.0 { return out; } } } return out; }
"###);
    }

    #[test]
    fn generate_shader_multi_effect() {
        test_render_type::<ShapeRender<RoundedRect, (Border, FillImage)>>("", "");
    }

    #[test]
    fn generate_shader_all_effect() {
        test_render_type::<ShapeRender<RoundedRect, (Border, FillColor, Shadow, Shadow)>>("", "");
    }

    #[test]
    fn generate_shader_multi_shape() {
        test_render_type::<(
            ShapeRender<RoundedRect, (Border, FillColor, Shadow)>,
            Transformed<ShapeRender<Circle, (FillColor, Shadow)>, Translation>,
        )>("", "");
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
                    move |mut commands: Commands,
                          mut ui_material: ResMut<Assets<ShaderAsset<R>>>| {
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
                            Color::WHITE * 0.5,
                            Color::BLUE.rgba_to_vec4() - Color::RED.rgba_to_vec4(),
                            Vec2::ONE.normalize() / 256.0,
                        ),
                        Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0),
                    )),
                ),
                shader_unit_test(
                    &temp_dir_path,
                    "rect_fill",
                    Vec2::splat(384.0),
                    Rect::new().with_effect(FillColor::new(Color::BLUE)),
                ),
            ],
        );

        std::fs::remove_dir_all(temp_dir_path).unwrap();
    }
}
