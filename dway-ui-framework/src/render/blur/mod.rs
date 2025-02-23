use std::borrow::Cow;

use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    ecs::{entity::EntityHashMap, query::QueryItem},
    render::{
        extract_component::ExtractComponent,
        mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            AddressMode, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d,
            FragmentState, MultisampleState, Operations, PipelineCache, PrimitiveState,
            RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
            SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType,
            SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor,
            TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
            TextureViewDescriptor, VertexState,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        view::ViewTarget,
        Extract, RenderApp, RenderSet,
    },
};

use super::layer_manager::{BlurMethod, BlurMethodKind, LayerCamera, LayerManager};
use crate::prelude::*;

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component, PartialEq)]
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
    type Out = Self;
    type QueryData = &'static Blur;
    type QueryFilter = With<Camera>;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}

#[derive(Component)]
pub struct BlurData {
    input: TextureView,
    textures: Vec<TextureView>,
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

#[derive(Resource, Default)]
pub struct ExtractedBlurCamera {
    changed: Vec<Entity>,
    removed: Vec<Entity>,
}

pub fn extract_layer_manager(
    layer_manager_query: Extract<Query<Ref<LayerManager>>>,
    mut removed: Extract<RemovedComponents<LayerCamera>>,
    mut extracted: ResMut<ExtractedBlurCamera>,
    mut commands: Commands,
) {
    extracted.removed.clear();
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
            if layer_manager.is_changed() {
                extracted.changed.push(layer_manager.base_layer.camera);
            }
        } else if layer_manager.is_changed() {
            extracted.removed.push(layer_manager.base_layer.camera);
        }
    }
    extracted.removed.extend(removed.read());
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PreparedBlurData(pub EntityHashMap<(Blur, BlurData)>);

pub fn prepare_blur_pipeline(
    query: Query<(Entity, &Blur)>,
    extracted: Res<ExtractedBlurCamera>,
    mut datas: ResMut<PreparedBlurData>,
    render_device: Res<RenderDevice>,
    pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlurPipeline>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    for removed_entity in &extracted.removed {
        datas.remove(removed_entity);
    }
    for (entity, blur) in query.iter_many(&extracted.changed) {
        if let Some((cached_blue, _)) = datas.get(&entity) {
            if cached_blue == blur {
                continue;
            }
        }
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
            continue;
        };
        datas.insert(
            entity,
            (
                blur.clone(),
                BlurData {
                    input: input_image.texture_view.clone(),
                    textures,
                    layout,
                    sampler,
                    pipeline_id,
                },
            ),
        );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct BlurLabel;

#[derive(Default)]
struct BlurNode;

impl ViewNode for BlurNode {
    type ViewQuery = (Entity, &'static ViewTarget, &'static Blur);

    fn run<'w>(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext<'w>,
        (entity, view_target, blur): bevy::ecs::query::QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let meshes = world.resource::<RenderAssets<RenderMesh>>();
        let blur_datas = world.resource::<PreparedBlurData>();
        let mesh_allocator = world.resource::<MeshAllocator>();

        let Some((_, blur_data)) = blur_datas.get(&entity) else {
            return Ok(());
        };
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

                    let mesh_asset_id = &blur.area.id();
                    let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id)
                    else {
                        return;
                    };
                    render_pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

                    match &mesh.buffer_info {
                        RenderMeshBufferInfo::Indexed {
                            index_format,
                            count,
                        } => {
                            let Some(index_buffer_slice) =
                                mesh_allocator.mesh_index_slice(mesh_asset_id)
                            else {
                                return;
                            };

                            render_pass.set_index_buffer(
                                index_buffer_slice.buffer.slice(..),
                                0,
                                *index_format,
                            );

                            render_pass.draw_indexed(
                                index_buffer_slice.range.start
                                    ..(index_buffer_slice.range.start + count),
                                vertex_buffer_slice.range.start as i32,
                                0..1,
                            );
                        }
                        RenderMeshBufferInfo::NonIndexed => {
                            render_pass.draw(vertex_buffer_slice.range, 0..1);
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
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedBlurCamera>()
                .init_resource::<PreparedBlurData>()
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
