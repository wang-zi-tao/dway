use std::{
    any::{type_name, TypeId},
    marker::PhantomData,
    ops::Range,
};

use bevy::{
    asset::UntypedAssetId,
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        query::ROQueryItem,
        system::{
            SystemParamItem, lifetimeless::{Read, SRes}
        },
    },
    math::{Affine2, FloatOrd},
    platform::collections::HashMap,
    render::{
        Extract, Render, RenderApp, RenderSet, RenderStartup, globals::{GlobalsBuffer, GlobalsUniform}, render_asset::{RenderAssetPlugin, RenderAssets}, render_phase::{
            AddRenderCommand, DrawFunctionId, DrawFunctions, PhaseItem, PhaseItemExtraIndex,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewSortedRenderPhases,
        }, render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, BufferUsages, IndexFormat, PipelineCache, RawBufferVec, SpecializedRenderPipelines, binding_types::uniform_buffer
        }, renderer::{RenderDevice, RenderQueue}, sync_world::MainEntity, view::{ExtractedView, ViewUniform, ViewUniformOffset, ViewUniforms}
    },
    ui_render::{
        PreparedUiMaterial, TransparentUi, UiCameraMap, UiCameraView, UiMaterialPipeline, init_ui_material_pipeline, stack_z_offsets
    },
};
use bytemuck::{Pod, Zeroable};
use wgpu::ShaderStages;

use crate::prelude::*;

pub const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

pub const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

pub struct UiNodeRenderPlugin;
impl Plugin for UiNodeRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<NodeIndex>()
                .init_resource::<UnTypedUiMaterialPipeline>()
                .init_resource::<UiBufferSet>()
                .init_resource::<UiBatchMap>()
                .add_systems(
                    Render,
                    (prepare_ui_nodes.in_set(RenderSet::PrepareBindGroups),),
                );
        }
    }
}

pub struct UiMaterialPlugin<M: UiMaterial>(PhantomData<M>);

impl<M: UiMaterial> Default for UiMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<M: UiMaterial<Data = ()>> Plugin for UiMaterialPlugin<M> {
    fn build(&self, app: &mut App) {
        app.init_asset::<M>()
            .register_type::<MaterialNode<M>>()
            .add_plugins((RenderAssetPlugin::<PreparedUiMaterial<M>>::default(),));

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(RenderStartup, init_ui_material_pipeline::<M>)
                .add_systems(ExtractSchedule, extract_ui_nodes::<M>)
                .add_systems(Render, queue_ui_nodes::<M>.in_set(RenderSet::Queue));
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let pipeline = UnTypedUiPipeline::from_world::<M>(render_app.world_mut());
            {
                let mut untyped_pipelines = render_app
                    .world_mut()
                    .resource_mut::<UnTypedUiMaterialPipeline>();
                untyped_pipelines
                    .pipelines
                    .insert(TypeId::of::<M>(), pipeline);
            }
            {
                let mut node_set = render_app.world_mut().resource_mut::<UiBufferSet>();
                node_set.vertex_buffer.insert(
                    TypeId::of::<M>(),
                    UiVertexList {
                        view_bind_group: None,
                    },
                );
            }
        }
    }
}

#[derive(Component)]
pub struct ExtractedNode {
    pub stack_index: u32,
    pub transform: Affine2,
    pub rect: Rect,
    pub clip: Option<Rect>,
    pub extracted_camera_entity: Entity,
    pub visiable: bool,
    pub main_entity: MainEntity,
}

#[derive(Component)]
pub struct ExtractedUntypedMaterial {
    pub asset_id: UntypedAssetId,
}

#[derive(Component)]
pub struct ExtractedMaterial<M: UiMaterial>(PhantomData<M>);

impl<M: UiMaterial> Default for ExtractedMaterial<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct NodeIndex(pub HashMap<(TypeId, MainEntity), Entity>);

#[derive(Resource, Default)]
pub struct UnTypedUiMaterialPipeline {
    pub pipelines: HashMap<TypeId, UnTypedUiPipeline>,
    pub draw_function_index: HashMap<DrawFunctionId, TypeId>,
}

pub fn extract_ui_nodes<M: UiMaterial>(
    node: Extract<
        Query<(
            Entity,
            Ref<ComputedNode>,
            Ref<UiGlobalTransform>,
            Ref<InheritedVisibility>,
            Ref<ComputedUiTargetCamera>,
            Ref<MaterialNode<M>>,
            Option<Ref<CalculatedClip>>,
        )>,
    >,
    mut extracted_node_query: Query<(&mut ExtractedNode, &mut ExtractedUntypedMaterial)>,
    mut node_index: ResMut<NodeIndex>,
    mut removed_node: Extract<RemovedComponents<Node>>,
    mut removed_clip: Extract<RemovedComponents<CalculatedClip>>,
    mut removed_target_camera: Extract<RemovedComponents<UiTargetCamera>>,
    camera_map: Extract<UiCameraMap>,
    mut commands: Commands,
) {
    let mut camera_mapper = camera_map.get_mapper();

    let entity_with_removed_component =
        Iterator::chain(removed_clip.read(), removed_target_camera.read())
            .collect::<EntityHashSet>();

    for node_entity in removed_node.read() {
        if let Some(node) = node_index.remove(&(TypeId::of::<M>(), MainEntity::from(node_entity))) {
            let _ = commands.get_entity(node).map(|mut c| c.despawn());
        }
    }

    for (main_entity, computed_node, transform, visibility, camera, material, clip) in node.iter() {
        let stack_index = computed_node.stack_index();

        let Some(extracted_camera_entity) = camera_mapper.map(&camera) else {
            continue;
        };

        let key = (TypeId::of::<M>(), MainEntity::from(main_entity));
        let entity = node_index.get(&key).cloned();

        if let Some((mut extracted_node_component, mut extracted_material)) =
            entity.and_then(|e| extracted_node_query.get_mut(e).ok())
        {
            extracted_node_component.stack_index = stack_index;
            if transform.is_changed() {
                extracted_node_component.transform = **transform;
            }
            if computed_node.is_changed() {
                extracted_node_component.rect = Rect {
                    min: Vec2::ZERO,
                    max: computed_node.size(),
                };
            }
            let entity_changed = entity_with_removed_component.contains(&main_entity);
            if entity_changed || clip.as_ref().map(|c| c.is_changed()).unwrap_or(false) {
                extracted_node_component.clip = clip.map(|c| c.clip);
            }
            if entity_changed || camera.is_changed() {
                extracted_node_component.extracted_camera_entity = extracted_camera_entity;
            }
            if entity_changed || visibility.is_changed() {
                extracted_node_component.visiable = visibility.get();
            }

            if entity_changed || material.is_changed() {
                extracted_material.asset_id = material.id().untyped();
            }
        } else {
            let extracted_node = ExtractedNode {
                stack_index,
                transform: **transform,
                rect: Rect {
                    min: Vec2::ZERO,
                    max: computed_node.size(),
                },
                clip: clip.map(|clip| clip.clip),
                extracted_camera_entity,
                visiable: visibility.get(),
                main_entity: MainEntity::from(main_entity),
            };
            let extracted_material = ExtractedUntypedMaterial {
                asset_id: material.id().untyped(),
            };
            let entity = commands
                .spawn((
                    extracted_node,
                    extracted_material,
                    ExtractedMaterial::<M>::default(),
                ))
                .id();
            node_index.insert(key, entity);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UiMaterialVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub size: [f32; 2],
    pub border_widths: [f32; 4],
    pub border_radius: [f32; 4],
}

pub struct UiBatch {
    pub range: Range<u32>,
    pub material: UntypedAssetId,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct UiBatchMap(pub EntityHashMap<UiBatch>);

structstruck::strike! {
    #[derive(Resource, SmartDefault)]
    pub struct UiBufferSet{
        #[default(RawBufferVec::new(BufferUsages::VERTEX))]
        pub vertices: RawBufferVec<UiMaterialVertex>,
        #[default(RawBufferVec::new(BufferUsages::INDEX))]
        pub indices: RawBufferVec<u32>,
        pub vertex_buffer: HashMap<TypeId,
            pub struct UiVertexList {
                pub view_bind_group: Option<BindGroup>,
            }
        >
    }
}

impl UnTypedUiPipeline {
    pub fn from_world<M: UiMaterial<Data = ()>>(world: &mut World) -> Self {
        let draw_functions = world.resource::<DrawFunctions<TransparentUi>>();
        let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            "ui_view_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        Self {
            view_layout,
            draw_function,
        }
    }
}

#[derive(Debug, Component)]
pub struct UnTypedUiPipeline {
    pub view_layout: BindGroupLayout,
    pub draw_function: DrawFunctionId,
}

pub fn queue_ui_nodes<M: UiMaterial<Data = ()>>(
    query: Query<(Entity, &ExtractedNode), With<ExtractedMaterial<M>>>,
    mut specialized_pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<&ExtractedView>,
    mut render_views: Query<&UiCameraView, With<ExtractedView>>,
) {
    let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();

    for (index, (render_entity, extracted_uinode)) in query.iter().enumerate() {
        let main_entity = extracted_uinode.main_entity;

        let Ok(default_camera_view) =
            render_views.get_mut(extracted_uinode.extracted_camera_entity)
        else {
            continue;
        };

        let Ok(view) = views.get_mut(default_camera_view.0) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let specialized_pipeline = specialized_pipelines.specialize(
            &pipeline_cache,
            &ui_material_pipeline,
            UiMaterialKey {
                hdr: view.hdr,
                // sample_count: msaa.samples() as i8,
                bind_group_data: (),
            },
        );

        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline: specialized_pipeline,
            entity: (render_entity, main_entity),
            sort_key: FloatOrd(extracted_uinode.stack_index as f32 + stack_z_offsets::MATERIAL),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: true,
        });
    }
}

pub fn prepare_ui_nodes(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_uinodes: Query<(&ExtractedNode, &ExtractedUntypedMaterial)>,
    pipelines: Res<UnTypedUiMaterialPipeline>,
    mut render_ui_nodes: ResMut<UiBufferSet>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut batches: ResMut<UiBatchMap>,
) {
    render_ui_nodes.vertices.clear();
    render_ui_nodes.indices.clear();
    batches.clear();
    if let (Some(view_binding), Some(globals_binding)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        let mut pedding_batch: Option<(UntypedAssetId, Entity, UiBatch)> = Default::default();
        let mut indices_index = 0;
        let mut vertices_index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_shader_handle = AssetId::<Image>::invalid().untyped();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                let entity = item.entity();
                if let Ok((extracted_uinode, extracted_material)) = extracted_uinodes.get(entity) {
                    let asset_id = extracted_material.asset_id;
                    let mut create_batch = || {
                        batch_item_index = item_index;
                        batch_shader_handle = asset_id;
                        (
                            asset_id,
                            entity,
                            UiBatch {
                                range: vertices_index..vertices_index,
                                material: asset_id,
                            },
                        )
                    };
                    let pedding_batch = pedding_batch.get_or_insert_with(&mut create_batch);
                    if pedding_batch.0 != asset_id {
                        let finished_batch = std::mem::replace(pedding_batch, create_batch());
                        batches.insert(finished_batch.1, finished_batch.2);
                    }

                    let uinode_rect = extracted_uinode.rect;
                    let rect_size = uinode_rect.size();
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        extracted_uinode
                            .transform
                            .transform_point2(pos * rect_size)
                            .extend(1.0)
                    });
                    let positions_diff = if let Some(clip) = extracted_uinode.clip {
                        [
                            Vec2::new(
                                f32::max(clip.min.x - positions[0].x, 0.),
                                f32::max(clip.min.y - positions[0].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[1].x, 0.),
                                f32::max(clip.min.y - positions[1].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[2].x, 0.),
                                f32::min(clip.max.y - positions[2].y, 0.),
                            ),
                            Vec2::new(
                                f32::max(clip.min.x - positions[3].x, 0.),
                                f32::min(clip.max.y - positions[3].y, 0.),
                            ),
                        ]
                    } else {
                        [Vec2::ZERO; 4]
                    };

                    let positions_clipped = [
                        positions[0] + positions_diff[0].extend(0.),
                        positions[1] + positions_diff[1].extend(0.),
                        positions[2] + positions_diff[2].extend(0.),
                        positions[3] + positions_diff[3].extend(0.),
                    ];

                    let transformed_rect_size =
                        extracted_uinode.transform.transform_vector2(rect_size);

                    if extracted_uinode.transform.x_axis[1] == 0.0
                        && (positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y)
                    {
                        continue;
                    }
                    let uvs = [
                        Vec2::new(
                            uinode_rect.min.x + positions_diff[0].x,
                            uinode_rect.min.y + positions_diff[0].y,
                        ),
                        Vec2::new(
                            uinode_rect.max.x + positions_diff[1].x,
                            uinode_rect.min.y + positions_diff[1].y,
                        ),
                        Vec2::new(
                            uinode_rect.max.x + positions_diff[2].x,
                            uinode_rect.max.y + positions_diff[2].y,
                        ),
                        Vec2::new(
                            uinode_rect.min.x + positions_diff[3].x,
                            uinode_rect.max.y + positions_diff[3].y,
                        ),
                    ]
                    .map(|pos| pos / uinode_rect.max);

                    for i in 0..4 {
                        render_ui_nodes.vertices.push(UiMaterialVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            size: extracted_uinode.rect.size().into(),
                            border_widths: [0.0; 4],
                            border_radius: [0.0; 4],
                        });
                    }

                    for &i in &QUAD_INDICES {
                        render_ui_nodes.indices.push(indices_index + i as u32);
                    }

                    indices_index += 4;
                    vertices_index += 6;
                    pedding_batch.2.range.end = vertices_index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else if let Some(pedding_batch) = pedding_batch.take() {
                    batches.insert(pedding_batch.1, pedding_batch.2);
                }
            }
        }

        if let Some(pedding_batch) = pedding_batch {
            batches.insert(pedding_batch.1, pedding_batch.2);
        }

        for (type_id, ui_meta) in &mut render_ui_nodes.vertex_buffer {
            let pipeline = &pipelines.pipelines[type_id];
            ui_meta.view_bind_group = Some(render_device.create_bind_group(
                "ui_material_view_bind_group",
                &pipeline.view_layout,
                &BindGroupEntries::sequential((view_binding.clone(), globals_binding.clone())),
            ));
        }

        render_ui_nodes
            .vertices
            .write_buffer(&render_device, &render_queue);
        render_ui_nodes
            .indices
            .write_buffer(&render_device, &render_queue);
    }
}

pub type DrawUiMaterial<M> = (
    SetItemPipeline,
    SetMatUiViewBindGroup<M, 0>,
    SetUiMaterialBindGroup<M, 1>,
    DrawUiMaterialNode<M>,
);

pub struct SetMatUiViewBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P> for SetMatUiViewBindGroup<M, I> {
    type ItemQuery = ();
    type Param = SRes<UiBufferSet>;
    type ViewQuery = Read<ViewUniformOffset>;

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        render_ui_nodes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            render_ui_nodes.into_inner().vertex_buffer[&TypeId::of::<M>()]
                .view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type ItemQuery = ();
    type Param = (SRes<RenderAssets<PreparedUiMaterial<M>>>, SRes<UiBatchMap>);
    type ViewQuery = ();

    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        (materials, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batches.get(&item.entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.into_inner().get(batch.material.typed()) else {
            debug!(material_type=%type_name::<M>(), "the ui material is not prepared");
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawUiMaterialNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMaterialNode<M> {
    type ItemQuery = ();
    type Param = (SRes<UiBufferSet>, SRes<UiBatchMap>);
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _: Option<()>,
        (ui_meta, batches): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batches.get(&item.entity()) else {
            return RenderCommandResult::Skip;
        };

        let ui_meta = ui_meta.into_inner();
        pass.set_vertex_buffer(0, ui_meta.vertices.buffer().unwrap().slice(..));
        pass.set_index_buffer(
            ui_meta.indices.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}
