pub mod effect;
pub mod fill;
pub mod shape;
pub mod transform;

use std::{
    any::{type_name, TypeId},
    collections::BTreeSet,
    marker::PhantomData,
    path::PathBuf,
};

use bevy::{
    asset::{io::embedded::EmbeddedAssetRegistry, load_internal_asset, uuid_handle},
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    render::{
        render_asset::RenderAssets,
        render_resource::{
            encase::{
                internal::{AlignmentValue, BufferMut, SizeValue, WriteInto, Writer},
                private::Metadata,
                UniformBuffer,
            },
            AsBindGroup, AsBindGroupError, BindGroupLayout, BindGroupLayoutEntry, BindingResources,
            BindingType, BufferBindingType, BufferInitDescriptor, BufferUsages,
            OwnedBindingResource, RenderPipelineDescriptor, SamplerBindingType, ShaderStages,
            ShaderType, TextureSampleType, TextureViewDimension, UnpreparedBindGroup,
        },
        renderer::RenderDevice,
        texture::{FallbackImage, GpuImage},
    },
    shader::ShaderRef,
};
use dway_ui_derive::Interpolation;

use self::{effect::Effect, shape::Shape, transform::Transform};
use crate::{prelude::*, render::ui_nodes::UiMaterialPlugin};

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
        if !self.uniforms.iter().any(|(_, k, _)| k == &name) {
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
        if !self.binding.iter().any(|(_, k, _)| k == &name) {
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
            .join("\n");
        let uniforms = self
            .uniforms
            .iter()
            .enumerate()
            .map(|(i, (a, k, t))| format!("@location({i}) {a} {k}: {t},"))
            .collect::<Vec<_>>()
            .join("\n");
        let bindings = self
            .binding
            .iter()
            .enumerate()
            .map(|(i, (attr, name, ty))| {
                format!("@group(1) @binding({}) {attr} var {name}: {ty};", i + 1)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let vertex_fields = self
            .vertex_fields
            .iter()
            .map(|(p, k, t)| format!("{p} {k}: {t},"))
            .collect::<Vec<_>>()
            .join("\n");
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
    @location(4) border_radius: vec4<f32>,
) -> VertexOutput {{
    var out: VertexOutput;
    out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);
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
    pub images: &'l RenderAssets<GpuImage>,
    pub fallback_image: &'l FallbackImage,
}
impl<'l> BindGroupBuilder<'l> {
    pub fn new(
        layout: &'l BindGroupLayout,
        render_device: &'l RenderDevice,
        images: &'l RenderAssets<GpuImage>,
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

    pub fn build(self) -> UnpreparedBindGroup {
        UnpreparedBindGroup {
            bindings: BindingResources(self.output),
        }
    }

    pub fn add_image(&mut self, image: &Handle<Image>) -> Result<(), AsBindGroupError> {
        let image = self
            .images
            .get(image)
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        self.output.push((
            self.binding_number(),
            OwnedBindingResource::TextureView(TextureViewDimension::D2, image.texture_view.clone()),
        ));
        self.output.push((
            self.binding_number(),
            OwnedBindingResource::Sampler(SamplerBindingType::NonFiltering, image.sampler.clone()),
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
    pub fn update_layout<T: ShaderType>(&mut self, value: &T) {
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

    pub fn write_uniform<T: ShaderType + WriteInto, B: BufferMut>(
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
    fn bind_group_layout_entries(_builder: &mut BindGroupLayoutBuilder) {
    }
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
            "
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
            "
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
            pos: "rect_position".to_string(),
            size: "rect_size".to_string(),
        };
        T::to_wgsl(&mut builder, &vars);
        builder.build()
    }

    fn id() -> String {
        (format!("{:?}", TypeId::of::<T>())).replace(':', "=")
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
        is_pod: true,
    };

    fn min_size() -> std::num::NonZeroU64 {
        Self::METADATA.min_size().0
    }

    fn size(&self) -> std::num::NonZeroU64 {
        let mut layout = UniformLayout::default();
        self.render.update_layout(&mut layout);
        let size = layout.alignment.round_up_size(SizeValue::from(
            layout.size.try_into().unwrap_or(1.try_into().unwrap()),
        ));
        size.0
    }

    fn assert_uniform_compat() {
        Self::UNIFORM_COMPAT_ASSERT()
    }
}

impl<T: Material> AsBindGroup for ShaderAsset<T> {
    type Data = ();
    type Param = (SRes<RenderAssets<GpuImage>>, SRes<FallbackImage>);

    fn label() -> Option<&'static str> {
        Some(type_name::<Self>())
    }

    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (images, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
        force_no_bindless: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let mut builder = BindGroupBuilder::new(layout, render_device, images, fallback_image);
        builder.add_uniform_buffer(self)?;
        BuildBindGroup::unprepared_bind_group(&self.render, &mut builder)?;
        Ok(builder.build())
    }

    fn bind_group_layout_entries(
        render_device: &RenderDevice,
        force_no_bindless: bool,
    ) -> Vec<BindGroupLayoutEntry>
    where
        Self: Sized,
    {
        let mut builder = BindGroupLayoutBuilder::new(render_device);
        <T as BuildBindGroup>::bind_group_layout_entries(&mut builder);
        builder.build()
    }

    fn bind_group_data(&self) -> Self::Data {
        ()
    }
}

impl<T: Material> UiMaterial for ShaderAsset<T> {
    fn vertex_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(Self::path().into())
    }

    fn specialize(descriptor: &mut RenderPipelineDescriptor, _key: UiMaterialKey<Self>) {
        let label = format!(
            "{} {:?}",
            type_name::<Self>(),
            descriptor.label.as_ref().map(|l| l.as_ref())
        );
        descriptor.label = Some(label.into());
    }
}

pub struct ShaderPlugin<T: Material>(PhantomData<T>);

impl<T: Material> ShaderPlugin<T> {
    pub fn add_into(self, app: &mut App) {
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
            let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
            let path: PathBuf = ShaderAsset::<T>::raw_path().into();
            let wgsl = ShaderAsset::<T>::to_wgsl();
            trace!("add shader: {path:?}\n{wgsl}");
            embedded.insert_asset(std::path::PathBuf::new(), &path, wgsl.into_bytes());
            if !app.is_plugin_added::<UiMaterialPlugin<ShaderAsset<T>>>() {
                app.add_plugins(UiMaterialPlugin::<ShaderAsset<T>>::default());
            }
        }
    }

    fn is_unique(&self) -> bool {
        false
    }
}

const FRAMEWORK_HANDLE: Handle<Shader> = uuid_handle!("8d709f36-e01d-11f0-a0a6-e7ddf390e70c");

pub struct ShaderFrameworkPlugin;
impl Plugin for ShaderFrameworkPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, FRAMEWORK_HANDLE, "framework.wgsl", Shader::from_wgsl);
    }
}
