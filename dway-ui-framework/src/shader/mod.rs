use std::{
    any::{type_name, TypeId},
    collections::BTreeSet,
    hash::Hash,
    marker::PhantomData,
    mem::size_of,
    path::PathBuf,
};

use crate::prelude::*;
use bevy::render::render_resource::encase::private::Metadata;
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
use encase::{
    internal::{AlignmentValue, BufferMut, SizeValue, Writer},
    DynamicUniformBuffer,
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
            .map(|(attr, name, ty)| format!("@group(1) {attr} var {name}: {ty};"))
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
        self.output.len() as u32 + 1
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
                    min_binding_size: None, // TODO
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
        impl<$first_elem: BuildBindGroup,$($elem: BuildBindGroup),* > BuildBindGroup for ($first_elem,$($elem),*){
            fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
                $first_elem::bind_group_layout_entries(builder);
                $($elem::bind_group_layout_entries(builder);)*
            }

            #[allow(non_snake_case)]
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

pub mod effect {
    use bevy::render::render_resource::encase::internal::WriteInto;
    use encase::{
        internal::{BufferMut, Writer},
        ShaderType,
    };

    use super::{fill::Fill, shape::Shape, BuildBindGroup, Expr, ShaderBuilder, ShaderVariables};
    use crate::prelude::*;

    pub trait Effect: BuildBindGroup {
        fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables);
    }

    #[derive(Clone)]
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
            builder.add_import("dway_ui_framework::shader::framework::sigmoid");
            builder.add_import("dway_ui_framework::shader::framework::mix_alpha");
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
    impl WriteInto for Shadow {
        fn write_into<B>(&self, writer: &mut Writer<B>)
        where
            B: BufferMut,
        {
            self.color.write_into(writer);
            self.offset.write_into(writer);
            self.margin.write_into(writer);
            self.radius.write_into(writer);
        }
    }

    #[derive(Clone)]
    pub struct Border {
        pub color: Color,
        pub width: f32,
    }

    impl Border {
        pub fn new(color: Color, width: f32) -> Self {
            Self { color, width }
        }
    }
    impl Effect for Border {
        fn to_wgsl<S: Shape>(_shape_ns: &str, builder: &mut ShaderBuilder, _var: &ShaderVariables) {
            let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
            let uniform_width = builder.get_uniform("width", "", "f32");
            builder.add_import("dway_ui_framework::shader::framework::mix_color");
            builder.add_import("dway_ui_framework::shader::framework::mix_alpha");
            let code = format!(
                "
                {{
                    let border_d = abs(shape_d + (0.5 - 1.0/16.0) * {uniform_width}) - 0.5 * {uniform_width};
                    if border_d < 0.5 {{
                        out = mix_alpha(out, mix_color({uniform_color}, border_d));
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
    impl BuildBindGroup for Border {
        fn update_layout(&self, layout: &mut super::UniformLayout) {
            layout.update_layout(&self.color);
            layout.update_layout(&self.width);
        }

        fn write_uniform<B: BufferMut>(
            &self,
            layout: &mut super::UniformLayout,
            writer: &mut Writer<B>,
        ) {
            layout.write_uniform(&self.color, writer);
            layout.write_uniform(&self.width, writer);
        }
    }

    impl<T: Fill> Effect for T {
        fn to_wgsl<S: Shape>(_shape_ns: &str, builder: &mut ShaderBuilder, var: &ShaderVariables) {
            let expr_color = T::to_wgsl(builder, var);
            builder.add_import("dway_ui_framework::shader::framework::mix_color");
            builder.add_import("dway_ui_framework::shader::framework::mix_alpha");
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

    macro_rules! impl_effect_for_tuple {
        () => { };
        ($first_elem:ident,$($elem:ident,)*) => {
            impl<$first_elem: Effect,$($elem: Effect),* > Effect for ($first_elem,$($elem),*){
                #[allow(non_snake_case)]
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

    #[derive(Clone)]
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

    #[derive(Clone)]
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

    #[derive(Clone)]
    pub struct FillImage {
        pub min_uv: Vec2,
        pub size_uv: Vec2,
        pub size: Vec2,
        pub image: Handle<Image>,
    }

    impl FillImage {
        pub fn new(min_uv: Vec2, size_uv: Vec2, size: Vec2, image: Handle<Image>) -> Self {
            Self {
                min_uv,
                size_uv,
                size,
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
    use super::{BuildBindGroup, Expr, ShaderBuilder, ShaderVariables};
    use crate::prelude::*;

    pub trait Shape: BuildBindGroup {
        fn register_uniforms(builder: &mut ShaderBuilder);
        fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr;
        fn to_normal_vector_wgsl(&self, _builder: &mut ShaderBuilder, _pos: Expr) -> Expr {
            format!("vec3(0.0, 0.0, 1.0)")
        }
    }

    #[derive(Clone, Default)]
    pub struct Circle {}

    impl Circle {
        pub fn new() -> Self {
            Self {}
        }
    }
    impl Shape for Circle {
        fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
            let ShaderVariables { pos, size } = var;
            builder.add_import("dway_ui_framework::shader::framework::circleSDF");
            format!("circleSDF({pos}, 0.5 * min({size}.x, {size}.y))")
        }

        fn register_uniforms(builder: &mut ShaderBuilder) {}
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

    #[derive(Clone, Default)]
    pub struct Rect {}

    impl Rect {
        pub fn new() -> Self {
            Self {}
        }
    }
    impl Shape for Rect {
        fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> Expr {
            let ShaderVariables { pos, size } = var;
            builder.add_import("dway_ui_framework::shader::framework::rectSDF");
            format!("rectSDF({pos}, {size})")
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

    #[derive(Clone)]
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
            builder.add_import("dway_ui_framework::shader::framework::boxSDF");
            format!("boxSDF({pos}, {size}, {uniform_radius})")
        }

        fn register_uniforms(builder: &mut ShaderBuilder) {
            builder.get_uniform("radius", "", "f32");
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

    #[derive(Clone, Default)]
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
            builder.add_import("dway_ui_framework::shader::framework::boxSDF");
            format!("boxSDF({pos}, {size}, 0.5 * min({size}.x, {size}.y))")
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

    use super::{BuildBindGroup, Expr, ShaderBuilder, ShaderVariables};
    pub trait Transform: BuildBindGroup {
        fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables;
    }

    #[derive(Clone)]
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

    #[derive(Clone)]
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
            builder.add_import("dway_ui_framework::shader::framework::sdf_rotation");
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

    #[derive(Clone)]
    pub struct Margins {
        pub margins: Vec4,
    }

    impl Margins {
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
}

#[derive(Clone)]
pub struct ShapeRender<S: Shape, E: Effect> {
    pub shape: S,
    pub effect: E,
}

impl<S: Shape, E: Effect> ShapeRender<S, E> {
    pub fn new(shape: S, effect: E) -> Self {
        Self { shape, effect }
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

#[derive(Clone)]
pub struct Transformed<S: Material, T: Transform> {
    pub render: S,
    pub transform: T,
}

impl<S: Material, T: Transform> Transformed<S, T> {
    pub fn new(render: S, transform: T) -> Self {
        Self { render, transform }
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

#[derive(Asset)]
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
        format!("dway_ui/render/gen/{}/render.wgsl", Self::id())
    }
    pub fn path() -> String {
        format!("embedded://dway_ui/render/gen/{}/render.wgsl", Self::id())
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

impl<T: Material> Default for ShaderPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Material> Plugin for ShaderPlugin<T> {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<UiMaterialPlugin<ShaderAsset<T>>>() {
            if !app.is_plugin_added::<ShaderFrameworkPlugin>() {
                app.add_plugins(ShaderFrameworkPlugin);
            }
            let embedded = app.world.resource_mut::<EmbeddedAssetRegistry>();
            let path: PathBuf = ShaderAsset::<T>::raw_path().into();
            let wgsl = ShaderAsset::<T>::to_wgsl();
            trace!("add shader: {path:?}\n{wgsl}");
            embedded.insert_asset(std::path::PathBuf::new(), &path, wgsl.into_bytes());
            app.add_plugins(UiMaterialPlugin::<ShaderAsset<T>>::default());
        }
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
            RenderPlugin,
        },
        sprite::SpritePlugin,
        ui::UiPlugin,
        winit::WinitPlugin,
    };
    use bevy_image_export::{
        ImageExportBundle, ImageExportPlugin, ImageExportSettings, ImageExportSource,
    };
    use failure::format_err;
    use lazy_static::lazy_static;
    use pretty_assertions::{assert_eq, assert_ne};
    use regex::Regex;
    use std::borrow::Cow;

    use crate::tests::compare_image;

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
        assert_eq!(path, except_path);
        assert_eq!(simplify_wgsl(&wgsl), simplify_wgsl(except_wgsl))
    }

    #[test]
    fn generate_shader_shape() {
        test_render_type::<ShapeRender<RoundedRect, FillColor>>("embedded://dway_ui/render/gen/TypeId { t= 278058722727597187056032458654139997086 }/render.wgsl",
        "
#import bevy_render::view::View
#import dway_ui_framework::framework::boxSDF
#import dway_ui_framework::framework::mix_color 
@group(0) @binding(0) var<uniform> view: View;
@group(1) @binding(0) var<uniform> rect: Settings;
struct Settings {
    shape_size: vec2<f32>,
    shape_radius: f32,
    effect_color: vec4<f32>,
}
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
};
@vertex
fn vertex( @location(0) vertex_position: vec3<f32>, @location(1) vertex_uv: vec2<f32>, @location(2) border_widths: vec4<f32>, ) -> VertexOutput {
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.uv = vertex_uv;
    out.border_widths = border_widths;
    return out;
}
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let d = boxSDF(in.position, rect.shape_size, rect.shape_radius);
    if d<0.5 {
        out = mix_alpha(out, mix_color(rect.effect_color, d));
        if out.a > 255.0/256.0 {
            return out;
        }
    }
}
");
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

    fn test_shader<R: Material>(
        test_asset_relative_path: &str,
        size: Vec2,
        shader: R,
    ) -> anyhow::Result<()> {
        let export_plugin = ImageExportPlugin::default();
        let export_threads = export_plugin.threads.clone();

        let mut app = App::default();
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<WinitPlugin>()
                .add(ScheduleRunnerPlugin::default())
                // .set(WinitPlugin {
                //     run_on_any_thread: true,
                // })
                .add(ShaderPlugin::<R>::default())
                .add(export_plugin),
        )
        .insert_resource(ClearColor(Color::WHITE));

        let temp_dir = tempdir::TempDir::new("dway_ui_framework_unit_test")?;
        // let temp_dir_path = temp_dir.path().to_string_lossy().to_string();
        let temp_dir_path = temp_dir.into_path();
        std::fs::create_dir_all(&temp_dir_path)?;
        info!("template folder: {temp_dir_path:?}");
        let temp_dir_path_clone = temp_dir_path.clone();

        app.add_systems(
            Startup,
            move |mut commands: Commands,
                  mut images: ResMut<Assets<Image>>,
                  mut export_sources: ResMut<Assets<ImageExportSource>>,
                  mut ui_material: ResMut<Assets<ShaderAsset<R>>>| {
                let output_texture_handle = {
                    let size = Extent3d {
                        width: (size.x * 1.5) as u32,
                        height: (size.y * 1.5) as u32,
                        ..default()
                    };
                    let mut export_texture = Image {
                        texture_descriptor: TextureDescriptor {
                            label: None,
                            size,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::Rgba8UnormSrgb,
                            mip_level_count: 1,
                            sample_count: 1,
                            usage: TextureUsages::COPY_DST
                                | TextureUsages::COPY_SRC
                                | TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                        ..default()
                    };
                    export_texture.resize(size);
                    images.add(export_texture)
                };

                let camera = commands
                    .spawn(Camera2dBundle {
                        camera: Camera {
                            target: RenderTarget::Image(output_texture_handle.clone()),
                            ..default()
                        },
                        ..default()
                    })
                    .id();

                let handle = ui_material.add(shader.clone());
                commands
                    .spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.),
                                height: Val::Percent(100.),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                flex_direction: FlexDirection::Column,
                                ..default()
                            },
                            ..default()
                        },
                        TargetCamera(camera),
                    ))
                    .with_children(|c| {
                        c.spawn(MaterialNodeBundle {
                            style: Style {
                                width: Val::Px(size.x),
                                height: Val::Px(size.y),
                                margin: UiRect::all(Val::Px(8.0)),
                                ..default()
                            },
                            material: handle,
                            ..default()
                        });
                    });

                commands.spawn(ImageExportBundle {
                    source: export_sources.add(output_texture_handle),
                    settings: ImageExportSettings {
                        output_dir: temp_dir_path_clone.clone().to_string_lossy().to_string(),
                        extension: "png".into(),
                    },
                });
            },
        );

        app.add_systems(
            Update,
            |frame: Res<FrameCount>, mut exit_event: EventWriter<AppExit>| {
                if frame.0 > 2 {
                    exit_event.send(AppExit);
                }
            },
        );
        app.run();
        export_threads.finish();

        let file = temp_dir_path
            .read_dir()?
            .next()
            .ok_or_else(|| anyhow!("Export image not found"))??;
        match compare_image(
            &file.path(),
            &PathBuf::from(test_asset_relative_path),
            &temp_dir_path,
        ) {
            Ok(Some(diff)) => {
                return Err(anyhow::anyhow!("image is different. diff image: {diff:?}"));
            }
            Ok(None) => {
                std::fs::remove_dir_all(temp_dir_path)?;
            }
            Err(e) => {
                return Err(e);
            }
        }
        Ok(())
    }

    #[test]
    fn test_shaders() {
        let results = [
            test_shader(
                "test/comparison_image/shader/circle_gradient_border_shadow.png",
                Vec2::splat(256.0),
                ShapeRender::new(
                    Circle::new(),
                    (
                        Border::new(Color::WHITE, 2.0),
                        Gradient::new(
                            Color::WHITE * 0.5,
                            Color::BLUE.rgba_to_vec4() - Color::RED.rgba_to_vec4(),
                            Vec2::ONE.normalize() / 256.0,
                        ),
                        Shadow::new(color!("#888888"), Vec2::ONE * 1.0, Vec2::ONE * 1.0, 2.0),
                    ),
                ),
            ),
            //     test_shader(
            //     "test/comparison_image/shader/rect_fill.png1",
            //     Vec2::new(128.0, 64.0),
            //     ShapeRender::new(Rect::new(), FillColor::new(Color::BLUE)),
            // ),
        ];
        for r in results {
            r.unwrap();
        }
    }
}
