use std::borrow::Cow;

use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::query::QueryItem,
    render::{
        Extract, RenderApp, RenderSet, camera::Viewport, extract_component::ExtractComponent, mesh::{RenderMesh, RenderMeshBufferInfo, VertexBufferLayout, allocator::MeshAllocator}, render_asset::RenderAssets, render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner}, render_resource::{
            AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d, FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor, TextureDimension, TextureFormat, TextureId, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor, VertexState, binding_types::{sampler, texture_2d, uniform_buffer}
        }, renderer::{RenderDevice, RenderQueue}, sync_world::{MainEntity, RenderEntity}, texture::GpuImage
    },
};
use serde::Deserialize;
use wgpu::{LoadOp, StoreOp, VertexFormat, VertexStepMode};

use super::layer_manager::{BlurMethod, BlurMethodKind, LayerCamera, LayerManager};
use crate::prelude::*;

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone)]]
    #[derive(Component, PartialEq)]
    pub struct Blur{
        area: Handle<Mesh>,
        shader: Handle<Shader>,
        blur_method: BlurMethod,
        size: UVec2,
        blur_input: Handle<Image>,
        blur_output: Handle<Image>,
        main_entity: MainEntity,
    }
}
impl ExtractComponent for Blur {
    type Out = Self;
    type QueryData = &'static Blur;
    type QueryFilter = With<Camera>;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

pub struct BlurPass {
    pub bind_group: BindGroup,
    pub output: TextureView,
    pub viewport: Option<Viewport>,
}

#[derive(Component)]
pub struct BlurData {
    output_image_id: TextureId,
    input: TextureView,
    passes: Vec<BlurPass>,
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

pub struct BlurPipeline {
    pub shader: Handle<Shader>,
    pub layout: BindGroupLayout,
}

impl SpecializedRenderPipeline for BlurPipeline {
    type Key = BlurMethodKind;

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // uv
                VertexFormat::Float32x2,
            ],
        );
        RenderPipelineDescriptor {
            label: Some("blur".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: Vec::new(),
                entry_point: Cow::from("vertex"),
                buffers: vec![vertex_layout],
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
            zero_initialize_workgroup_memory: false,
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
    layer_manager_query: Extract<Query<(Ref<LayerManager>, RenderEntity)>>,
    mut removed: Extract<RemovedComponents<LayerCamera>>,
    mut camera_query: Query<&mut Blur>,
    mut commands: Commands,
) {
    for (layer_manager, render_entity) in layer_manager_query.iter() {
        if layer_manager.blur_enable {
            if layer_manager.is_changed() {
                let blur = Blur {
                    size: layer_manager.size,
                    area: layer_manager.blur_layer.area.clone(),
                    blur_method: layer_manager.blur_layer.blur_method,
                    blur_output: layer_manager.blur_layer.blur_image.clone(),
                    shader: layer_manager.blur_layer.shader.clone(),
                    blur_input: layer_manager.blur_layer.layer.background_image.clone(),
                    main_entity: MainEntity::from(layer_manager.base_layer.camera),
                };
                if let Ok(mut old_value) = camera_query.get_mut(render_entity) {
                    old_value.set_if_neq(blur);
                } else {
                    commands.entity(render_entity).insert(blur);
                }
            }
        } else if layer_manager.is_changed() {
            if let Ok(mut e) = commands.get_entity(render_entity) {
                e.remove::<Blur>();
            }
        }
    }
    for render_entity in removed.read() {
        if let Ok(mut e) = commands.get_entity(render_entity) {
            e.remove::<Blur>();
        }
    }
}

pub fn prepare_blur_pipeline(
    mut query: Query<(Entity, Ref<Blur>, Option<&mut BlurData>)>,
    mut removed: RemovedComponents<Blur>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlurPipeline>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    for removed_entity in removed.read() {
        if let Ok(mut e) = commands.get_entity(removed_entity) {
            e.remove::<BlurData>();
        }
    }
    for (entity, blur, blur_data) in query.iter_mut() {
        let Some(output_image) = gpu_images.get(blur.blur_output.id()) else {
            continue;
        };

        let output_gpu_image_changed = blur_data
            .as_ref()
            .map_or(true, |data| data.output_image_id != output_image.texture.id());

        if blur_data.is_some() && !blur.is_changed() && !output_gpu_image_changed {
            continue;
        }

        let layout = render_device.create_bind_group_layout(
            "blur",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT | ShaderStages::VERTEX_FRAGMENT,
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

        let Some(input_image) = gpu_images.get(blur.blur_input.id()) else {
            continue;
        };
        let mut source_texture = input_image.texture_view.clone();

        let mut passes = vec![];
        blur.blur_method
            .foreach_layer(blur.size.as_vec2(), |uniform, is_output| {
                let dest_texture = if is_output {
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

                let mut uniform_buffer = DynamicUniformBuffer::<BlurUniform>::default();
                {
                    let Some(mut writer) =
                        uniform_buffer.get_writer(1, &render_device, &render_queue)
                    else {
                        return;
                    };
                    writer.write(&uniform);
                }

                let Some(uniform_binding) = uniform_buffer.binding() else {
                    return;
                };
                let bind_group = render_device.create_bind_group(
                    "blur_post_process_bind_group",
                    &layout,
                    &BindGroupEntries::sequential((&source_texture, &sampler, uniform_binding)),
                );

                passes.push(BlurPass {
                    bind_group,
                    output: dest_texture.clone(),
                    viewport: None,
                });

                source_texture = dest_texture;
            });
        let new_data = BlurData {
            input: input_image.texture_view.clone(),
            layout,
            sampler,
            pipeline_id,
            passes,
            output_image_id: output_image.texture.id(),
        };
        if let Some(mut blur_data) = blur_data {
            *blur_data = new_data;
        } else {
            commands.entity(entity).insert(new_data);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BlurLabel;

#[derive(Default)]
struct BlurNode;

impl ViewNode for BlurNode {
    type ViewQuery = (&'static Blur, &'static BlurData);

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (blur, blur_data): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let meshes = world.resource::<RenderAssets<RenderMesh>>();
        let mesh_allocator = world.resource::<MeshAllocator>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(blur_data.pipeline_id) else {
            return Ok(());
        };
        let Some(mesh) = meshes.get(blur.area.id()) else {
            return Ok(());
        };

        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(&blur.area.id()) else {
            return Ok(());
        };

        for BlurPass {
            bind_group,
            output,
            viewport,
        } in &blur_data.passes
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("blur_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_render_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

            if let Some(viewport) = viewport {
                render_pass.set_camera_viewport(viewport);
            }

            match &mesh.buffer_info {
                RenderMeshBufferInfo::Indexed {
                    index_format,
                    count,
                } => {
                    let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(&blur.area.id())
                    else {
                        continue;
                    };

                    render_pass.set_index_buffer(
                        index_buffer_slice.buffer.slice(..),
                        0,
                        *index_format,
                    );

                    render_pass.draw_indexed(
                        index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                        vertex_buffer_slice.range.start as i32,
                        0..1,
                    );
                }
                RenderMeshBufferInfo::NonIndexed => {
                    render_pass.draw(vertex_buffer_slice.range.clone(), 0..1);
                }
            }
        }

        Ok(())
    }
}

pub struct BlurRenderPlugin;
impl Plugin for BlurRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
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
                    (Node2d::MsaaWriteback, BlurLabel, Node2d::StartMainPass),
                );
        }
    }
}
