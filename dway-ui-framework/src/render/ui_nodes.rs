use std::{
    any::{type_name, TypeId},
    cell::Cell,
    hash::Hash,
    marker::PhantomData,
    ops::Range,
};

use bevy::{
    app::DynEq,
    asset::UntypedAssetId,
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::FloatOrd,
    render::{
        globals::{GlobalsBuffer, GlobalsUniform},
        mesh::PrimitiveTopology,
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctionId, DrawFunctions, PhaseItem, PhaseItemExtraIndex,
            RenderCommand, RenderCommandResult, SetItemPipeline, TrackedRenderPass,
            ViewSortedRenderPhases,
        },
        render_resource::{
            binding_types::uniform_buffer, AsBindGroupError, BindGroup, BindGroupEntries,
            BindGroupLayout, BindGroupLayoutEntries, BlendState, BufferUsages, BufferVec,
            ColorTargetState, ColorWrites, FragmentState, FrontFace, MultisampleState,
            OwnedBindingResource, PipelineCache, PolygonMode, PrimitiveState, RawBufferVec,
            RenderPipelineDescriptor, ShaderRef, ShaderStages, SpecializedRenderPipeline,
            SpecializedRenderPipelines, TextureFormat, VertexBufferLayout, VertexFormat,
            VertexState, VertexStepMode,
        },
        renderer::{RenderDevice, RenderQueue},
        texture::{BevyDefault, FallbackImage, GpuImage},
        view::{ExtractedView, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
        Extract, Render, RenderApp, RenderSet,
    },
    ui::{ExtractedUiNode, ExtractedUiNodes, RenderUiSystem, TransparentUi, UiStack},
    utils::{hashbrown::hash_map::Entry, HashMap, HashSet},
};
use bevy_relationship::reexport::SmallVec;
use bytemuck::{Pod, Zeroable};
use smallbox::{space::S2, SmallBox};

use super::ui_material::{QUAD_INDICES, QUAD_VERTEX_POSITIONS, UI_MATERIAL_SHADER_HANDLE};
use crate::{prelude::*, widgets::scroll::ui_scroll_render};

pub struct UiNodeRenderPlugin;
impl Plugin for UiNodeRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedUiNodeSet>()
                .init_resource::<RenderUiMaterialSet>()
                .init_resource::<RenderUiNodeSet>()
                .init_resource::<SpecializedRenderPipelines<UiPipeline>>()
                .add_systems(ExtractSchedule, extract_ui_nodes)
                .add_systems(
                    Render,
                    (
                        queue_ui_nodes.in_set(RenderSet::Queue),
                        prepare_ui_nodes.in_set(RenderSet::PrepareBindGroups),
                    ),
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
        app.init_asset::<M>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterials<M>>()
                .add_systems(
                    ExtractSchedule,
                    (extract_ui_materials::<M>, extract_ui_node_handle::<M>)
                        .after(extract_ui_nodes),
                )
                .add_systems(
                    Render,
                    prepare_ui_materials::<M>.in_set(RenderSet::PrepareAssets),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let pipeline = UiPipeline::new::<M>(render_app.world_mut());
            {
                let mut material_set = render_app.world_mut().resource_mut::<RenderUiMaterialSet>();
                material_set
                    .draw_function_index
                    .insert(pipeline.draw_function, TypeId::of::<M>());
                material_set.pipelines.insert(TypeId::of::<M>(), pipeline);
            }
            {
                let mut node_set = render_app.world_mut().resource_mut::<RenderUiNodeSet>();
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

structstruck::strike! {
    #[derive(Resource, Default)]
    pub struct ExtractedUiNodeSet{
        pub removed_node: Vec<Entity>,
        pub nodes: EntityHashMap<
            pub struct ExtractedNode{
                pub stack_index: usize,
                pub transform: Mat4,
                pub rect: Rect,
                pub clip: Option<Rect>,
                pub camera_entity: Entity,
            }>,
        pub node_materials: HashMap<(Entity, TypeId), UntypedAssetId>
    }
}

type UiMaterialSpecializeFn = fn(&mut RenderPipelineDescriptor, UiMaterialKey);

structstruck::strike! {
    #[derive(Resource, Default)]
    pub struct RenderUiMaterialSet{
        pub instantces: HashMap<UntypedAssetId,
            pub struct PreparedUiMaterialInstant {
                pub bindings: Vec<(u32, OwnedBindingResource)>,
                pub bind_group: BindGroup,
                // pub key: UiMaterial::Key
            }
        >,
        pub pipelines: HashMap<TypeId,
            #[derive(Debug)]
            pub struct UiPipeline {
                pub specialize: UiMaterialSpecializeFn,
                pub ui_layout: BindGroupLayout,
                pub view_layout: BindGroupLayout,
                pub vertex_shader: Option<Handle<Shader>>,
                pub fragment_shader: Option<Handle<Shader>>,
                pub draw_function: DrawFunctionId,
            }
        >,
        pub draw_function_index: HashMap<DrawFunctionId, TypeId>,
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct UiMaterialVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub size: [f32; 2],
    pub border_widths: [f32; 4],
}

#[derive(Component)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub material: UntypedAssetId,
}

structstruck::strike! {
    #[derive(Resource, SmartDefault)]
    pub struct RenderUiNodeSet{
        #[default(RawBufferVec::new(BufferUsages::VERTEX))]
        pub vertices: RawBufferVec<UiMaterialVertex>,
        pub vertex_buffer: HashMap<TypeId,
            pub struct UiVertexList {
                pub view_bind_group: Option<BindGroup>,
            }
        >
    }
}

structstruck::strike! {
    #[derive(Clone, Hash, PartialEq, Eq)]
    pub struct UiMaterialKey {
        pub type_id: TypeId,
        pub hdr: bool,
        pub sample_count: i8,
        // pub key: UiMaterial::Key
    }
}

pub fn specialize<M: UiMaterial<Data = ()>>(
    descriptor: &mut RenderPipelineDescriptor,
    key: UiMaterialKey,
) {
    M::specialize(
        descriptor,
        bevy::ui::UiMaterialKey {
            hdr: key.hdr,
            bind_group_data: (),
            sample_count: key.sample_count,
        },
    );
}

impl UiPipeline {
    pub fn new<M: UiMaterial<Data = ()>>(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let draw_functions = world.resource::<DrawFunctions<TransparentUi>>();
        let ui_layout = M::bind_group_layout(render_device);
        let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();

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
            ui_layout,
            view_layout,
            vertex_shader: match M::vertex_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            fragment_shader: match M::fragment_shader() {
                ShaderRef::Default => None,
                ShaderRef::Handle(handle) => Some(handle),
                ShaderRef::Path(path) => Some(asset_server.load(path)),
            },
            specialize: specialize::<M>,
            draw_function,
        }
    }
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiMaterialKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // size
                VertexFormat::Float32x2,
                // border_widths
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        let mut descriptor = RenderPipelineDescriptor {
            vertex: VertexState {
                shader: UI_MATERIAL_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: UI_MATERIAL_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.ui_layout.clone()],
            push_constant_ranges: Vec::new(),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.sample_count as u32,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("dway_untyped_ui_material_pipeline".into()),
        };
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }

        descriptor.layout = vec![self.view_layout.clone(), self.ui_layout.clone()];

        (self.specialize)(&mut descriptor, key);

        descriptor
    }
}

pub fn extract_ui_nodes(
    mut extracted: ResMut<ExtractedUiNodeSet>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<
            (
                Ref<Node>,
                Ref<Style>,
                Ref<GlobalTransform>,
                Ref<ViewVisibility>,
                Option<Ref<CalculatedClip>>,
                Option<Ref<TargetCamera>>,
            ),
            Without<BackgroundColor>,
        >,
    >,
    mut removed_node: RemovedComponents<Node>,
    mut removed_clip: RemovedComponents<CalculatedClip>,
    mut removed_target_camera: RemovedComponents<TargetCamera>,
    default_ui_camera: Extract<DefaultUiCamera>,
) {
    let default_ui_camera = default_ui_camera.get();
    let entity_with_removed_component = removed_clip
        .read()
        .chain(removed_target_camera.read())
        .collect::<EntityHashSet>();

    for node_entity in removed_node.read() {
        extracted.removed_node.push(node_entity);
    }

    for (stack_index, &node_entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((node, style, transform, visibility, clip, camera)) =
            uinode_query.get(node_entity)
        {
            if !visibility.get() {
                extracted.removed_node.push(node_entity);
                extracted.nodes.remove(&node_entity);
                continue;
            }

            let Some(camera_entity) = camera.as_ref().map(|x| x.entity()).or(default_ui_camera)
            else {
                continue;
            };

            match extracted.nodes.entry(node_entity) {
                Entry::Occupied(mut o) => {
                    let extracted_node = o.get_mut();
                    extracted_node.stack_index = stack_index;
                    if transform.is_changed() {
                        extracted_node.transform = transform.compute_matrix();
                    }
                    if node.is_changed() {
                        extracted_node.rect = Rect {
                            min: Vec2::ZERO,
                            max: node.size(),
                        };
                    }
                    let entity_changed = entity_with_removed_component.contains(&node_entity);
                    if entity_changed || clip.as_ref().map(|c| c.is_changed()).unwrap_or(false) {
                        extracted_node.clip = clip.map(|c| c.clip);
                    }
                    if entity_changed || camera.as_ref().map(|c| c.is_changed()).unwrap_or(false) {
                        extracted_node.camera_entity = camera_entity;
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(ExtractedNode {
                        stack_index,
                        transform: transform.compute_matrix(),
                        rect: Rect {
                            min: Vec2::ZERO,
                            max: node.size(),
                        },
                        clip: clip.map(|clip| clip.clip),
                        camera_entity,
                    });
                }
            }
        }
    }
}

pub fn extract_ui_node_handle<M: UiMaterial>(
    query: Extract<Query<(Entity, Ref<Handle<M>>, Ref<ViewVisibility>), With<Node>>>,
    mut removed: Extract<RemovedComponents<Handle<M>>>,
    mut extracted: ResMut<ExtractedUiNodeSet>,
) {
    let ExtractedUiNodeSet { node_materials, .. } = &mut *extracted;

    for (entity, handle, visibility) in &query {
        if !visibility.get() {
            node_materials.remove(&(entity, TypeId::of::<M>()));
            continue;
        }
        node_materials.insert((entity, TypeId::of::<M>()), handle.id().untyped());
    }

    for entity in removed.read() {
        node_materials.remove(&(entity, TypeId::of::<M>()));
    }
}

#[derive(Resource)]
pub struct ExtractedUiMaterials<M: UiMaterial> {
    extracted: Vec<(AssetId<M>, M)>,
    removed: Vec<AssetId<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub fn extract_ui_materials<M: UiMaterial>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.read() {
        #[allow(clippy::match_same_arms)]
        match event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                changed_assets.insert(*id);
            }
            AssetEvent::Removed { id } => {
                changed_assets.remove(id);
                removed.push(*id);
            }
            AssetEvent::Unused { .. } => {}
            AssetEvent::LoadedWithDependencies { .. } => {
                // TODO: handle this
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedUiMaterials {
        extracted: extracted_assets,
        removed,
    });
}

pub fn prepare_ui_materials<M: UiMaterial>(
    mut prepare_next_frame: Local<Vec<(AssetId<M>, M)>>,
    mut extracted_assets: ResMut<ExtractedUiMaterials<M>>,
    mut render_materials: ResMut<RenderUiMaterialSet>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
) {
    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.instantces.remove(&removed.untyped());
    }

    let queued_assets = std::mem::take(&mut *prepare_next_frame);
    for (handle, material) in Iterator::chain(
        std::mem::take(&mut extracted_assets.extracted).into_iter(),
        queued_assets,
    ) {
        match prepare_ui_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &render_materials,
        ) {
            Ok(prepared_asset) => {
                render_materials
                    .instantces
                    .insert(handle.untyped(), prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.push((handle, material));
            }
        }
    }
}

fn prepare_ui_material<M: UiMaterial>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<GpuImage>,
    fallback_image: &Res<FallbackImage>,
    render_materials: &RenderUiMaterialSet,
) -> Result<PreparedUiMaterialInstant, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &render_materials.pipelines[&TypeId::of::<M>()].ui_layout,
        render_device,
        images,
        fallback_image,
    )?;
    Ok(PreparedUiMaterialInstant {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        // key: prepared.data,
    })
}

pub fn queue_ui_nodes(
    extracted: Res<ExtractedUiNodeSet>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderUiMaterialSet>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(Entity, &ExtractedView)>,
    msaa: Res<Msaa>,
) {
    for ((entity, _), asset_id) in extracted.node_materials.iter() {
        let extracted_uinode = &extracted.nodes[entity];
        let Ok((view_entity, view)) = views.get_mut(extracted_uinode.camera_entity) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        // let Some(material) = render_materials.instantces.get(handle) else {
        //     continue;
        // };

        let Some(ui_pipeline) = render_materials.pipelines.get(&asset_id.type_id()) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            ui_pipeline,
            UiMaterialKey {
                hdr: view.hdr,
                sample_count: msaa.samples() as i8,
                type_id: asset_id.type_id(),
            },
        );

        transparent_phase.items.reserve(extracted.nodes.len());
        transparent_phase.add(TransparentUi {
            draw_function: ui_pipeline.draw_function,
            pipeline,
            entity: *entity,
            sort_key: (
                FloatOrd(extracted_uinode.stack_index as f32),
                entity.index(),
            ),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::NONE,
        });
    }
}

pub fn prepare_ui_nodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    extracted_uinodes: Res<ExtractedUiNodeSet>,
    render_materials: Res<RenderUiMaterialSet>,
    mut render_ui_nodes: ResMut<RenderUiNodeSet>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    render_ui_nodes.vertices.clear();
    if let (Some(view_binding), Some(globals_binding)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);
        let mut pedding_batch: Option<(UntypedAssetId, Entity, UiBatch)> = Default::default();
        let mut index = 0;

        for mut ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_shader_handle = AssetId::<Image>::invalid().untyped();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                let entity = item.entity;
                if let Some(extracted_uinode) = extracted_uinodes.nodes.get(&item.entity) {
                    let Some(&asset_typeid) = render_materials
                        .draw_function_index
                        .get(&item.draw_function)
                    else {
                        continue;
                    };
                    let asset_id = extracted_uinodes.node_materials[&(entity, asset_typeid)];
                    let mut create_batch = || {
                        batch_item_index = item_index;
                        batch_shader_handle = asset_id;
                        (
                            asset_id,
                            entity,
                            UiBatch {
                                range: index..index,
                                material: asset_id,
                            },
                        )
                    };
                    let pedding_batch = pedding_batch.get_or_insert_with(&mut create_batch);
                    if pedding_batch.0 != asset_id {
                        let finished_batch = std::mem::replace(pedding_batch, create_batch());
                        batches.push((finished_batch.1, finished_batch.2));
                    }

                    let uinode_rect = extracted_uinode.rect;
                    let rect_size = uinode_rect.size().extend(1.0);
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        (extracted_uinode.transform * (pos * rect_size).extend(1.0)).xyz()
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
                        extracted_uinode.transform.transform_vector3(rect_size);

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

                    for i in QUAD_INDICES {
                        render_ui_nodes.vertices.push(UiMaterialVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            size: extracted_uinode.rect.size().into(),
                            border_widths: [0.0; 4],
                        });
                    }

                    index += QUAD_INDICES.len() as u32;
                    pedding_batch.2.range.end = index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else if let Some(pedding_batch) = pedding_batch.take() {
                    batches.push((pedding_batch.1, pedding_batch.2));
                }
            }
        }

        if let Some(pedding_batch) = pedding_batch {
            batches.push((pedding_batch.1, pedding_batch.2));
        }

        for (type_id, ui_meta) in &mut render_ui_nodes.vertex_buffer {
            let pipeline = &render_materials.pipelines[type_id];
            ui_meta.view_bind_group = Some(render_device.create_bind_group(
                "ui_material_view_bind_group",
                &pipeline.view_layout,
                &BindGroupEntries::sequential((view_binding.clone(), globals_binding.clone())),
            ));
        }

        render_ui_nodes
            .vertices
            .write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
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
    type Param = SRes<RenderUiNodeSet>;
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
    type ItemQuery = Read<UiBatch>;
    type Param = SRes<RenderUiMaterialSet>;
    type ViewQuery = ();

    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: Option<ROQueryItem<'_, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material_handle) = material_handle else {
            return RenderCommandResult::Failure;
        };
        let Some(material) = materials
            .into_inner()
            .instantces
            .get(&material_handle.material)
        else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawUiMaterialNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMaterialNode<M> {
    type ItemQuery = Read<UiBatch>;
    type Param = SRes<RenderUiNodeSet>;
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiBatch>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}
