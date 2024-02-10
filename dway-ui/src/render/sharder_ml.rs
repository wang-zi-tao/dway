use std::{
    any::{type_name, TypeId},
    collections::BTreeSet,
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
};

use bevy::{
    asset::io::embedded::EmbeddedAssetRegistry,
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

use crate::prelude::*;

use self::{effect::Effect, shape::Shape, transform::Transform};

type Ident = String;
type Expr = String;
type Stat = String;

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
    pub fn get_var(&mut self, name: &str, attr: &str, ty: &str) -> Ident {
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
            .map(|(a, k, t)| format!("{a} {k}: {t},"))
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
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
{vertex_fields}
}};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) border_widths: vec4<f32>,
) -> VertexOutput {{
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.uv = vertex_uv;
    out.border_widths = border_widths;
{vertex_inner}
    return out;
}}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {{
    var out = vec4(0.0);
    let position = in.position.xy;
{fragment_inner}
    return out;
}}
"
        )
    }
}

pub struct BindGroupBuilder<'l> {
    pub output: Vec<(u32, OwnedBindingResource)>,
    pub buffer: UniformBuffer<Vec<u8>>,
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
            buffer: UniformBuffer::new(vec![]),
            layout,
            render_device,
            images,
            fallback_image,
        }
    }

    pub fn binding_number(&self) -> u32 {
        self.output.len() as u32 + 1
    }

    pub fn build(mut self) -> UnpreparedBindGroup<()> {
        self.output.insert(
            0,
            (
                0,
                OwnedBindingResource::Buffer(self.render_device.create_buffer_with_data(
                    &BufferInitDescriptor {
                        label: None,
                        usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                        contents: self.buffer.as_ref(),
                    },
                )),
            ),
        );
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

    pub fn add_uniform<V: ShaderType + WriteInto>(&mut self, value: &V) {
        self.buffer.write(value).unwrap();
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

pub trait BuildBindGroup: Clone + Send + Sync + 'static {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder);
    fn unprepared_bind_group(&self, builder: &mut BindGroupBuilder)
        -> Result<(), AsBindGroupError>;
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
        }
        impl_build_bind_group_for_tuple!($($elem,)*);
    };
}
impl_build_bind_group_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);

pub mod effect {
    use super::{fill::Fill, shape::Shape, BuildBindGroup, Expr, ShaderBuilder};
    use crate::prelude::*;

    pub trait Effect: BuildBindGroup {
        fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, pos: Expr);
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
        fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, pos: Expr) {
            todo!()
        }
    }
    impl BuildBindGroup for Shadow {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.color);
            builder.add_uniform(&self.offset);
            builder.add_uniform(&self.margin);
            builder.add_uniform(&self.radius);
            Ok(())
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
        fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, pos: Expr) {
            todo!()
        }
    }
    impl BuildBindGroup for Border {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.color);
            builder.add_uniform(&self.width);
            Ok(())
        }
    }

    impl<T: Fill> Effect for T {
        fn to_wgsl<S: Shape>(_shape_ns: &str, builder: &mut ShaderBuilder, _pos: Expr) {
            let expr_color = T::to_wgsl(builder, format!("pos"));
            builder.add_import("dway_ui::shapes::mix_color");
            builder.add_import("dway_ui::shapes::mix_alpha");
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
                fn to_wgsl<S: Shape>(shape_ns: &str, builder: &mut ShaderBuilder, pos: Expr) {
                    builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::to_wgsl::<S>(shape_ns, builder, pos.clone()));
                    $( builder.in_new_namespace(stringify!($elem), |builder|$elem::to_wgsl::<S>(shape_ns, builder, pos.clone())); )*
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
    use crate::prelude::*;

    pub trait Fill: BuildBindGroup {
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr;
    }

    #[derive(Clone)]
    pub struct Gradient {
        pub color: Color,
        pub delta_color: Color,
        pub direction: Vec2,
    }

    impl Gradient {
        pub fn new(color: Color, delta_color: Color, direction: Vec2) -> Self {
            Self {
                color,
                delta_color,
                direction,
            }
        }
    }
    impl Fill for Gradient {
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
            let uniform_delta_color = builder.get_uniform("delta_color", "", "vec4<f32>");
            let uniform_direction = builder.get_uniform("direction", "", "vec2<f32>");
            format!("({uniform_color} + {uniform_delta_color} * dot({pos}, {uniform_direction}))")
        }
    }
    impl BuildBindGroup for Gradient {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            builder.add_uniform(&self.color);
            builder.add_uniform(&self.delta_color);
            builder.add_uniform(&self.direction);
            Ok(())
        }
    }

    #[derive(Clone)]
    pub struct FillColor {
        pub color: Color,
    }

    impl FillColor {
        pub fn new(color: Color) -> Self {
            Self { color }
        }
    }
    impl Fill for FillColor {
        fn to_wgsl(builder: &mut ShaderBuilder, _pos: Expr) -> Expr {
            let uniform_color = builder.get_uniform("color", "", "vec4<f32>");
            uniform_color
        }
    }
    impl BuildBindGroup for FillColor {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), AsBindGroupError> {
            builder.add_uniform(&self.color);
            Ok(())
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
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_min_uv = builder.get_uniform("min_uv", "", "vec2<f32>");
            let uniform_size_uv = builder.get_uniform("size_uv", "", "vec2<f32>");
            let var_image_texture = builder.get_var("image_texture", "", "texture_2d<f32>");
            let var_image_sampler = builder.get_var("image_sampler", "", "sampler");
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
            builder.add_uniform(&self.min_uv);
            builder.add_uniform(&self.size_uv);
            Ok(())
        }
    }
}

pub mod shape {
    use super::{BuildBindGroup, Expr, ShaderBuilder};
    use crate::prelude::*;

    pub trait Shape: BuildBindGroup {
        fn register_uniforms(builder: &mut ShaderBuilder);
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr;
        fn to_normal_vector_wgsl(&self, _builder: &mut ShaderBuilder, _pos: Expr) -> Expr {
            format!("vec3(0.0, 0.0, 1.0)")
        }
    }

    #[derive(Clone)]
    pub struct Circle {
        pub r: f32,
    }

    impl Circle {
        pub fn new(r: f32) -> Self {
            Self { r }
        }
    }
    impl Shape for Circle {
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_radius = builder.get_uniform("radius", "", "f32");
            builder.add_import("dway_ui::shapes::circleSDF");
            format!("circleSDF({pos}, {uniform_radius})")
        }

        fn register_uniforms(builder: &mut ShaderBuilder) {
            builder.get_uniform("radius", "", "f32");
        }
    }
    impl BuildBindGroup for Circle {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.r);
            Ok(())
        }
    }

    #[derive(Clone)]
    pub struct Rect {
        pub size: Vec2,
    }
    impl Shape for Rect {
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_size = builder.get_uniform("size", "", "vec2<f32>");
            builder.add_import("dway_ui::shapes::rectSDF");
            format!("rectSDF({pos}, {uniform_size})")
        }

        fn register_uniforms(builder: &mut ShaderBuilder) {
            builder.get_uniform("size", "", "vec2<f32>");
        }
    }
    impl BuildBindGroup for Rect {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.size);
            Ok(())
        }
    }

    #[derive(Clone)]
    pub struct RoundedRect {
        pub size: Vec2,
        pub corner: f32,
    }
    impl Shape for RoundedRect {
        fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_size = builder.get_uniform("size", "", "vec2<f32>");
            let uniform_radius = builder.get_uniform("radius", "", "f32");
            builder.add_import("dway_ui::shapes::boxSDF");
            format!("boxSDF({pos}, {uniform_size}, {uniform_radius})")
        }

        fn register_uniforms(builder: &mut ShaderBuilder) {
            builder.get_uniform("size", "", "vec2<f32>");
            builder.get_uniform("radius", "", "f32");
        }
    }
    impl BuildBindGroup for RoundedRect {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.size);
            builder.add_uniform(&self.corner);
            Ok(())
        }
    }
}

pub mod transform {
    use crate::prelude::*;

    use super::{BuildBindGroup, Expr, ShaderBuilder};
    pub trait Transform: BuildBindGroup {
        fn transform(builder: &mut ShaderBuilder, pos: Expr) -> Expr;
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
        fn transform(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            let uniform_offset = builder.get_uniform("offset", "", "vec2<f32>");
            format!("({pos}-{uniform_offset})")
        }
    }
    impl BuildBindGroup for Translation {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.offset);
            Ok(())
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
        fn transform(builder: &mut ShaderBuilder, pos: Expr) -> Expr {
            builder.add_import("dway_ui::shapes::sdf_rotation");
            let uniform_rotation = builder.get_uniform("rotation", "", "f32");
            format!("sdf_rotation({pos}, {uniform_rotation})")
        }
    }
    impl BuildBindGroup for Rotation {
        fn bind_group_layout_entries(builder: &mut super::BindGroupLayoutBuilder) {}

        fn unprepared_bind_group(
            &self,
            builder: &mut super::BindGroupBuilder,
        ) -> Result<(), bevy::render::render_resource::AsBindGroupError> {
            builder.add_uniform(&self.rotation);
            Ok(())
        }
    }
}

pub trait Render: BuildBindGroup {
    fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr);
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

impl<S: Shape, E: Effect> Render for ShapeRender<S, E> {
    fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) {
        let ns = builder.new_namespace("shape");
        builder.in_namespace(&ns, |builder| {
            let expr_d = S::to_wgsl(builder, format!("shape_pos"));
            let code = format!("
                let shape_pos = {pos};
                let shape_d = {expr_d};
            ");
            builder.fragment_inner += &*code;
            S::register_uniforms(builder) });
        builder.in_new_namespace("effect", |builder| E::to_wgsl::<S>(&ns, builder, format!("shape_pos")));
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
}

#[derive(Clone)]
pub struct Transformed<S: Render, T: Transform> {
    pub render: S,
    pub transform: T,
}

impl<S: Render, T: Transform> Transformed<S, T> {
    pub fn new(render: S, transform: T) -> Self {
        Self { render, transform }
    }
}
impl<S: Render, T: Transform> BuildBindGroup for Transformed<S, T> {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
        S::bind_group_layout_entries(builder);
        T::bind_group_layout_entries(builder);
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        self.render.unprepared_bind_group(builder)?;
        self.transform.unprepared_bind_group(builder)?;
        Ok(())
    }
}
impl<S: Render, T: Transform> Render for Transformed<S, T> {
    fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) {
        let transformed = T::transform(builder, pos);
        S::to_wgsl(builder, transformed);
    }
}

macro_rules! impl_render_for_tuple {
    () => { };
    ($first_elem:ident,$($elem:ident,)*) => {
        impl<$first_elem: Render,$($elem: Render),* > Render for ($first_elem,$($elem),*){
            #[allow(non_snake_case)]
            fn to_wgsl(builder: &mut ShaderBuilder, pos: Expr) {
                builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::to_wgsl(builder, pos.clone()));
                $( builder.in_new_namespace(stringify!($elem), |builder|$elem::to_wgsl(builder, pos.clone())); )*
            }
        }
        impl_render_for_tuple!($($elem,)*);
    };
}
impl_render_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);

#[derive(Asset)]
pub struct ShaderAsset<T: Render> {
    pub render: T,
}

impl<T: Render> From<T> for ShaderAsset<T> {
    fn from(value: T) -> Self {
        Self { render: value }
    }
}

impl<T: Render> ShaderAsset<T> {
    pub fn new(render: T) -> Self {
        Self { render }
    }

    pub fn to_wgsl() -> String {
        let mut builder = ShaderBuilder::default();
        T::to_wgsl(&mut builder, format!("position"));
        builder.build()
    }
    fn id() -> String {
        (format!("{:?}",TypeId::of::<T>())).replace(|c: char| c == ':', "=")
    }
    pub fn raw_path() -> String {
        format!("dway_ui/render/gen/{}/render.wgsl", Self::id())
    }
    pub fn path() -> String {
        format!("embedded://dway_ui/render/gen/{}/render.wgsl", Self::id())
    }
}

impl<T: Render> Clone for ShaderAsset<T> {
    fn clone(&self) -> Self {
        Self {
            render: self.render.clone(),
        }
    }
}

impl<T: Render> TypePath for ShaderAsset<T> {
    fn type_path() -> &'static str {
        type_name::<Self>()
    }

    fn short_type_path() -> &'static str {
        type_name::<Self>()
    }
}

impl<T: Render> AsBindGroup for ShaderAsset<T> {
    type Data = ();

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        images: &RenderAssets<Image>,
        fallback_image: &FallbackImage,
    ) -> std::prelude::v1::Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        let mut builder = BindGroupBuilder::new(layout, render_device, images, fallback_image);
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

impl<T: Render> UiMaterial for ShaderAsset<T> {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn specialize(_descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {}
}

pub struct ShaderPlugin<T: Render>(PhantomData<T>);

impl<T: Render> Default for ShaderPlugin<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Render> Plugin for ShaderPlugin<T> {
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
}

#[cfg(test)]
pub mod test {
    use bevy::{core_pipeline::CorePipelinePlugin, render::RenderPlugin, ui::UiPlugin};
    use lazy_static::lazy_static;
    use pretty_assertions::{assert_eq, assert_ne};
    use regex::Regex;
    use std::borrow::Cow;

    use self::{
        effect::{Border, Shadow},
        fill::{FillColor, FillImage},
        shape::{Circle, RoundedRect},
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

    fn test_render_type<R: Render>(except_path: &str, except_wgsl: &str) {
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
#import dway_ui::shapes::boxSDF
#import dway_ui::shapes::mix_color 
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
}
