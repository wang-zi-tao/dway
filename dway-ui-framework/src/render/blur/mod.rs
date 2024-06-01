use crate::prelude::*;
use bevy::{
    core_pipeline::{
        core_2d::graph::{Core2d, Node2d},
        core_3d::graph::{Core3d, Node3d},
        fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    },
    ecs::query::QueryItem,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_graph::{RenderGraphApp, RenderLabel, ViewNode, ViewNodeRunner},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d, FragmentState,
            MultisampleState, Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType,
            SamplerDescriptor, ShaderStages, ShaderType, Texture, TextureDescriptor,
            TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
            TextureViewDescriptor,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, GpuImage},
        view::ViewTarget,
        RenderApp, RenderSet,
    },
};

use super::layer_manager::{BlurMethod, LayerManager};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component)]
    pub struct Blur{
        area: Handle<Mesh>,
        shader: Handle<Shader>,
        blur_method: BlurMethod,
        size: UVec2,
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
    textures: Vec<Texture>,
    layout: BindGroupLayout,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

impl BlurMethod {
    fn foreach_layer(&self, size: UVec2, mut f: impl FnMut(BlurUniform, bool)) {
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
                for i in 0..(*layer / 2) as u32 {
                    f(
                        BlurUniform {
                            radius: *radius,
                            layer: i,
                            stage: 0,
                            size: size / u32::pow(2, i),
                        },
                        i + 1 == (*layer / 2) as u32,
                    );
                }
                for i in (0..(*layer / 2) as u32).rev() {
                    f(
                        BlurUniform {
                            radius: *radius,
                            layer: i,
                            stage: 1,
                            size: size / u32::pow(2, (*layer as u32) - i - 1),
                        },
                        i + 1 == *layer as u32,
                    );
                }
            }
        }
    }

    fn fragment_name(&self) -> &str {
        match self {
            BlurMethod::Kawase { .. } => "fragment_kwase",
            BlurMethod::Dual { .. } => "fragment_dual",
        }
    }
}

#[derive(Component, Default, Clone, Copy, ExtractComponent, ShaderType)]
struct BlurUniform {
    radius: f32,
    layer: u32,
    stage: u32,
    size: UVec2,
}

pub fn prepare_layer_manager(layer_manager_query: Query<&LayerManager>, mut commands: Commands) {
    for layer_manager in layer_manager_query.iter() {
        if layer_manager.blur_enable {
            commands
                .entity(layer_manager.base_layer.camera)
                .insert(Blur {
                    area: layer_manager.blur_layer.layer.area.clone(),
                    blur_method: layer_manager.blur_layer.blur_method,
                    size: layer_manager.size,
                    blur_output: layer_manager.blur_layer.layer.surface.clone(),
                    shader: layer_manager.blur_layer.shader.clone(),
                });
        }
    }
}

pub fn prepare_blur_pipeline(
    query: Query<(Entity, &Blur)>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
    pipeline_cache: ResMut<PipelineCache>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    for (entity, blur) in query.iter() {
        let layout = render_device.create_bind_group_layout(
            "post_process_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<BlurUniform>(false),
                ),
            ),
        );
        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let shader = blur.shader.clone();
        let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("post_process_pipeline".into()),
            layout: vec![layout.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader,
                shader_defs: vec![],
                entry_point: blur.blur_method.fragment_name().to_string().into(),
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
        });

        let mut textures = vec![];
        blur.blur_method
            .foreach_layer(blur.size, |uniform, is_output| {
                let texture = if is_output {
                    let gpu_image = gpu_images.get(blur.blur_output.id()).unwrap();
                    gpu_image.texture.clone()
                } else {
                    render_device.create_texture(&TextureDescriptor {
                        label: Some("blur layer"),
                        size: Extent3d {
                            width: uniform.size.x,
                            height: uniform.size.y,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Bgra8UnormSrgb,
                        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    })
                };
                textures.push(texture);
            });
        commands.entity(entity).insert(BlurData {
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
        let Some(pipeline) = pipeline_cache.get_render_pipeline(blur_data.pipeline_id) else {
            return Ok(());
        };
        let mut source_texture = view_target.main_texture_view().clone();
        let mut dest_texture = view_target.main_texture_view().clone();
        let mut texture_iter = blur_data.textures.iter();

        blur.blur_method.foreach_layer(blur.size, |uniform, _| {
            dest_texture = texture_iter
                .next()
                .unwrap()
                .create_view(&TextureViewDescriptor::default());

            let mut uniform_buffer = DynamicUniformBuffer::<BlurUniform>::default();
            {
                let Some(mut writer) = uniform_buffer.get_writer(1, render_device, render_queue)
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
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
            render_pass.draw(0..3, 0..1);

            source_texture = dest_texture.clone();
        });
        Ok(())
    }
}

pub struct PostProcessingPlugin;
impl Plugin for PostProcessingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<Blur>::default());
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
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
