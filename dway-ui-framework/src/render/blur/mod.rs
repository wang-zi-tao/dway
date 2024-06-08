use std::borrow::Cow;

use crate::prelude::*;
use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::{fullscreen_shader_vertex_state, FULLSCREEN_SHADER_HANDLE},
    },
    ecs::query::QueryItem,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::GpuBufferInfo,
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            AddressMode, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d,
            FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
            SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
            SpecializedRenderPipeline, SpecializedRenderPipelines, Texture, TextureDescriptor,
            TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
            TextureViewDescriptor, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, GpuImage},
        view::ViewTarget,
        Extract, RenderApp, RenderSet,
    },
    utils::HashMap,
};

use super::layer_manager::{BlurMethod, BlurMethodKind, LayerManager};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component)]
    pub struct Blur{
        area: Handle<Mesh>,
        shader: Handle<Shader>,
        blur_method: BlurMethod,
        size: UVec2,
        blur_input: Handle<Image>,
        blur_output: Handle<Image>,
    }
}
impl ExtractComponent for Blur {
    type QueryData = &'static Blur;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

#[derive(Component)]
struct BlurData {
    input: TextureView,
    textures: Vec<TextureView>,
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

struct BlurPipeline {
    pub shader: Handle<Shader>,
    pub layout: BindGroupLayout,
}

impl SpecializedRenderPipeline for BlurPipeline {
    type Key = BlurMethodKind;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("blur".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                entry_point: Cow::from("vertex"),
                buffers: Vec::new(),
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: vec![],
                entry_point: Cow::from("fragment"),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        }
    }
}

impl BlurMethod {
    fn foreach_layer(&self, size: Vec2, mut f: impl FnMut(BlurUniform, bool)) {
        match self {
            BlurMethod::Kawase { layer, radius } => {
                for i in 0..*layer as u32 {
                    f(
                        BlurUniform {
                            radius: *radius,
                            layer: i,
                            stage: 0,
                            size,
                        },
                        i + 1 == *layer as u32,
                    );
                }
            }
            BlurMethod::Dual { layer, radius } => {
                for i in 1..=(*layer / 2) as u32 {
                    f(
                        BlurUniform {
                            radius: *radius,
                            layer: i,
                            stage: 0,
                            size: size / u32::pow(2, i) as f32,
                        },
                        false,
                    );
                }
                for i in (1 + (*layer / 2) as u32)..=*layer as u32 {
                    f(
                        BlurUniform {
                            radius: *radius,
                            layer: i,
                            stage: 1,
                            size: size / u32::pow(2, (*layer as u32) - i) as f32,
                        },
                        i == *layer as u32,
                    );
                }
            }
        }
    }
}

#[derive(Component, Default, Clone, Copy, Debug, ExtractComponent, ShaderType)]
struct BlurUniform {
    radius: f32,
    layer: u32,
    stage: u32,
    size: Vec2,
}

pub fn extract_layer_manager(
    layer_manager_query: Extract<Query<&LayerManager>>,
    mut commands: Commands,
) {
    for layer_manager in layer_manager_query.iter() {
        if layer_manager.blur_enable {
            let blur = Blur {
                size: layer_manager.size,
                area: layer_manager.blur_layer.layer.area.clone(),
                blur_method: layer_manager.blur_layer.blur_method,
                blur_output: layer_manager.blur_layer.blur_image.clone(),
                shader: layer_manager.blur_layer.shader.clone(),
                blur_input: layer_manager.blur_layer.layer.background_image.clone(),
            };
            commands
                .get_or_spawn(layer_manager.base_layer.camera)
                .insert(blur);
        }
    }
}

pub fn prepare_blur_pipeline(
    query: Query<(Entity, &Blur)>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
    pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlurPipeline>>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    for (entity, blur) in query.iter() {
        let layout = render_device.create_bind_group_layout(
            "blur",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<BlurUniform>(false),
                ),
            ),
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("blur"),
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            ..Default::default()
        });
        let shader = blur.shader.clone();
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &BlurPipeline {
                shader,
                layout: layout.clone(),
            },
            blur.blur_method.kind(),
        );

        let mut textures = vec![];
        blur.blur_method
            .foreach_layer(blur.size.as_vec2(), |uniform, is_output| {
                let texture = if is_output {
                    let gpu_image = gpu_images.get(blur.blur_output.id()).unwrap();
                    gpu_image.texture_view.clone()
                } else {
                    let texture = render_device.create_texture(&TextureDescriptor {
                        label: Some("blur layer"),
                        size: Extent3d {
                            width: uniform.size.x as u32,
                            height: uniform.size.y as u32,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Rgba8UnormSrgb,
                        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    });
                    texture.create_view(&TextureViewDescriptor::default())
                };
                textures.push(texture);
            });
        let Some(input_image) = gpu_images.get(blur.blur_input.id()) else {
            continue
        };
        commands.entity(entity).insert(BlurData {
            input: input_image.texture_view.clone(),
            textures,
            layout,
            sampler,
            pipeline_id,
        });
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BlurLabel;

#[derive(Default)]
struct BlurNode;

impl ViewNode for BlurNode {
    type ViewQuery = (&'static ViewTarget, &'static Blur, &'static BlurData);

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (view_target, blur, blur_data): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let meshes = world.resource::<RenderAssets<Mesh>>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(blur_data.pipeline_id) else {
            return Ok(());
        };
        let Some(mesh) = meshes.get(blur.area.id()) else {
            return Ok(());
        };

        let mut source_texture = blur_data.input.clone();
        let mut texture_iter = blur_data.textures.iter();

        blur.blur_method
            .foreach_layer(blur.size.as_vec2(), |uniform, _| {
                let dest_texture = texture_iter.next().unwrap().clone();

                let mut uniform_buffer = DynamicUniformBuffer::<BlurUniform>::default();
                {
                    let Some(mut writer) =
                        uniform_buffer.get_writer(1, render_device, render_queue)
                    else {
                        return;
                    };
                    writer.write(&uniform);
                }
                let Some(uniform_binding) = uniform_buffer.binding() else {
                    return;
                };
                let bind_group = render_context.render_device().create_bind_group(
                    "blur_post_process_bind_group",
                    &blur_data.layout,
                    &BindGroupEntries::sequential((
                        &source_texture,
                        &blur_data.sampler,
                        uniform_binding,
                    )),
                );

                {
                    let mut render_pass =
                        render_context.begin_tracked_render_pass(RenderPassDescriptor {
                            label: Some("blur_post_process_pass"),
                            color_attachments: &[Some(RenderPassColorAttachment {
                                view: &dest_texture,
                                resolve_target: None,
                                ops: Operations::default(),
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });

                    render_pass.set_render_pipeline(pipeline);
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));

                    match &mesh.buffer_info {
                        GpuBufferInfo::Indexed {
                            buffer,
                            index_format,
                            count,
                        } => {
                            render_pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                            render_pass.draw_indexed(0..*count, 0, 0..1);
                        }
                        GpuBufferInfo::NonIndexed => {
                            render_pass.draw(0..mesh.vertex_count, 0..1);
                        }
                    }
                }

                source_texture = dest_texture;
            });
        Ok(())
    }
}

pub struct PostProcessingPlugin;
impl Plugin for PostProcessingPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpecializedRenderPipelines<BlurPipeline>>()
                .add_systems(ExtractSchedule, extract_layer_manager)
                .add_systems(
                    bevy::render::Render,
                    prepare_blur_pipeline.in_set(RenderSet::PrepareAssets),
                )
                .add_render_graph_node::<ViewNodeRunner<BlurNode>>(Core2d, BlurLabel)
                .add_render_graph_edges(
                    Core2d,
                    (
                        Node2d::Tonemapping,
                        BlurLabel,
                        Node2d::EndMainPassPostProcessing,
                    ),
                );
        }
    }
}
