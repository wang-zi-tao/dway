use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy::{
    ecs::{
        query::ROQueryItem,
        storage::SparseSet,
        system::{
            lifetimeless::{Read, SRes},
            SystemParamItem,
        },
    },
    math::FloatOrd,
    render::{
        extract_component::ExtractComponentPlugin,
        globals::GlobalsBuffer,
        render_asset::{RenderAssetPlugin, RenderAssets},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{
            BindGroup, BindGroupEntries, BufferUsages, PipelineCache, RawBufferVec,
            SpecializedRenderPipeline, SpecializedRenderPipelines,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity, TemporaryRenderEntity},
        view::{ExtractedView, ViewUniformOffset, ViewUniforms},
        Extract, Render, RenderApp, RenderSet,
    },
    ui::{stack_z_offsets, PreparedUiMaterial, TransparentUi, UiMaterialPipeline, UiMaterialVertex, UiStack},
    utils::HashSet,
    window::PrimaryWindow,
};

use crate::prelude::*;

pub const UI_MATERIAL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10074188772096983955);

/// Adds the necessary ECS resources and render logic to enable rendering entities using the given
/// [`UiMaterial`] asset type (which includes [`UiMaterial`] types).
pub struct UiMaterialPlugin<M: UiMaterial>(PhantomData<M>);

impl<M: UiMaterial> Default for UiMaterialPlugin<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub(crate) const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

pub(crate) const QUAD_VERTEX_POSITIONS: [Vec3; 4] = [
    Vec3::new(-0.5, -0.5, 0.0),
    Vec3::new(0.5, -0.5, 0.0),
    Vec3::new(0.5, 0.5, 0.0),
    Vec3::new(-0.5, 0.5, 0.0),
];

pub(crate) fn resolve_border_thickness(value: Val, parent_width: f32, viewport_size: Vec2) -> f32 {
    match value {
        Val::Auto => 0.,
        Val::Px(px) => px.max(0.),
        Val::Percent(percent) => (parent_width * percent / 100.).max(0.),
        Val::Vw(percent) => (viewport_size.x * percent / 100.).max(0.),
        Val::Vh(percent) => (viewport_size.y * percent / 100.).max(0.),
        Val::VMin(percent) => (viewport_size.min_element() * percent / 100.).max(0.),
        Val::VMax(percent) => (viewport_size.max_element() * percent / 100.).max(0.),
    }
}

impl<M: UiMaterial> Plugin for UiMaterialPlugin<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.init_asset::<M>().add_plugins((
            ExtractComponentPlugin::<MaterialNode<M>>::extract_visible(),
            RenderAssetPlugin::<PreparedUiMaterial<M>>::default(),
        ));
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMaterial<M>>()
                .init_resource::<ExtractedUiMaterials<M>>()
                .init_resource::<ExtractedUiMaterialNodes<M>>()
                .init_resource::<UiMaterialMeta<M>>()
                .init_resource::<SpecializedRenderPipelines<UiMaterialPipeline<M>>>()
                .add_systems(
                    ExtractSchedule,
                    (extract_ui_materials::<M>, extract_ui_nodes::<M>),
                )
                .add_systems(
                    Render,
                    (
                        queue_ui_material_nodes::<M>.in_set(RenderSet::Queue),
                        prepare_uimaterial_nodes::<M>.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiMaterialPipeline<M>>();
        }
    }
}

#[derive(Resource)]
pub struct UiMaterialMeta<M: UiMaterial> {
    vertices: RawBufferVec<UiMaterialVertex>,
    view_bind_group: Option<BindGroup>,
    marker: PhantomData<M>,
}

impl<M: UiMaterial> Default for UiMaterialMeta<M> {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            view_bind_group: Default::default(),
            marker: PhantomData,
        }
    }
}

#[derive(Component)]
pub struct UiMaterialBatch<M: UiMaterial> {
    /// The range of vertices inside the [`UiMaterialMeta`]
    pub range: Range<u32>,
    pub material: AssetId<M>,
    pub camera: Entity,
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
    type Param = SRes<UiMaterialMeta<M>>;
    type ViewQuery = Read<ViewUniformOffset>;

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            ui_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}

pub struct SetUiMaterialBindGroup<M: UiMaterial, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial, const I: usize> RenderCommand<P>
    for SetUiMaterialBindGroup<M, I>
{
    type ItemQuery = Read<UiMaterialBatch<M>>;
    type Param = SRes<RenderAssets<PreparedUiMaterial<M>>>;
    type ViewQuery = ();

    fn render<'w>(
        _item: &P,
        _view: (),
        material_handle: Option<ROQueryItem<'_, Self::ItemQuery>>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(material_handle) = material_handle else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials.into_inner().get(material_handle.material) else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawUiMaterialNode<M>(PhantomData<M>);
impl<P: PhaseItem, M: UiMaterial> RenderCommand<P> for DrawUiMaterialNode<M> {
    type ItemQuery = Read<UiMaterialBatch<M>>;
    type Param = SRes<UiMaterialMeta<M>>;
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiMaterialBatch<M>>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, ui_meta.into_inner().vertices.buffer().unwrap().slice(..));
        pass.draw(batch.range.clone(), 0..1);
        RenderCommandResult::Success
    }
}

pub struct ExtractedUiMaterialNode<M: UiMaterial> {
    pub stack_index: usize,
    pub transform: Mat4,
    pub rect: Rect,
    pub border: [f32; 4],
    pub material: AssetId<M>,
    pub clip: Option<Rect>,
    pub camera_entity: Entity,
    pub main_entity: Entity,
}

#[derive(Resource)]
pub struct ExtractedUiMaterialNodes<M: UiMaterial> {
    pub uinodes: SparseSet<Entity, ExtractedUiMaterialNode<M>>,
}

impl<M: UiMaterial> Default for ExtractedUiMaterialNodes<M> {
    fn default() -> Self {
        Self {
            uinodes: Default::default(),
        }
    }
}

pub fn extract_ui_nodes<M: UiMaterial>(
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    materials: Extract<Res<Assets<M>>>,
    ui_stack: Extract<Res<UiStack>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &Node,
            &GlobalTransform,
            &MaterialNode<M>,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
        )>,
    >,
    windows: Extract<Query<&Window, With<PrimaryWindow>>>,
    ui_scale: Extract<Res<UiScale>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    render_entity_lookup: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    let default_single_camera = default_ui_camera.get();

    let ui_logical_viewport_size = windows
        .get_single()
        .map(|window| Vec2::new(window.resolution.width(), window.resolution.height()))
        .unwrap_or(Vec2::ZERO)
        // The logical window resolution returned by `Window` only takes into account the window scale factor and not `UiScale`,
        // so we have to divide by `UiScale` to get the size of the UI viewport.
        / ui_scale.0;
    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((
            entity,
            computed_node,
            uinode,
            transform,
            handle,
            view_visibility,
            clip,
            camera,
        )) = uinode_query.get(*entity)
        {
            let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_single_camera)
            else {
                continue;
            };

            let Ok(camera_entity) = render_entity_lookup.get(camera_entity) else {
                continue;
            };

            // skip invisible nodes
            if !view_visibility.get() {
                continue;
            }

            // Skip loading materials
            if !materials.contains(handle) {
                continue;
            }

            // Both vertical and horizontal percentage border values are calculated based on the width of the parent node
            // <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
            let parent_width = computed_node.size().x;
            let left = resolve_border_thickness(
                uinode.border.left,
                parent_width,
                ui_logical_viewport_size,
            ) / computed_node.size().x;
            let right = resolve_border_thickness(
                uinode.border.right,
                parent_width,
                ui_logical_viewport_size,
            ) / computed_node.size().x;
            let top =
                resolve_border_thickness(uinode.border.top, parent_width, ui_logical_viewport_size)
                    / computed_node.size().y;
            let bottom = resolve_border_thickness(
                uinode.border.bottom,
                parent_width,
                ui_logical_viewport_size,
            ) / computed_node.size().y;

            extracted_uinodes.uinodes.insert(
            commands.spawn(TemporaryRenderEntity).id(),
                ExtractedUiMaterialNode {
                    stack_index,
                    transform: transform.compute_matrix(),
                    material: handle.id(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: computed_node.size(),
                    },
                    border: [left, right, top, bottom],
                    clip: clip.map(|clip| clip.clip),
                    camera_entity,
                main_entity: entity.into(),
                },
            );
        };
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_uimaterial_nodes<M: UiMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiMaterialMeta<M>>,
    mut extracted_uinodes: ResMut<ExtractedUiMaterialNodes<M>>,
    view_uniforms: Res<ViewUniforms>,
    globals_buffer: Res<GlobalsBuffer>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let (Some(view_binding), Some(globals_binding)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        let mut batches: Vec<(Entity, UiMaterialBatch<M>)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_material_view_bind_group",
            &ui_material_pipeline.view_layout,
            &BindGroupEntries::sequential((view_binding, globals_binding)),
        ));
        let mut index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_shader_handle = AssetId::invalid();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(extracted_uinode) = extracted_uinodes.uinodes.get(item.entity()) {
                    let mut existing_batch = batches
                        .last_mut()
                        .filter(|_| batch_shader_handle == extracted_uinode.material);

                    if existing_batch.is_none()
                        || existing_batch.as_ref().map(|(_, b)| b.camera)
                            != Some(extracted_uinode.camera_entity)
                    {
                        batch_item_index = item_index;
                        batch_shader_handle = extracted_uinode.material;

                        let new_batch = UiMaterialBatch {
                            range: index..index,
                            material: extracted_uinode.material,
                            camera: extracted_uinode.camera_entity,
                        };

                        batches.push((item.entity(), new_batch));

                        existing_batch = batches.last_mut();
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

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if extracted_uinode.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
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
                        ui_meta.vertices.push(UiMaterialVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            size: extracted_uinode.rect.size().into(),
                            border_widths: extracted_uinode.border,
                        });
                    }

                    index += QUAD_INDICES.len() as u32;
                    existing_batch.unwrap().1.range.end = index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else {
                    batch_shader_handle = AssetId::invalid();
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
    extracted_uinodes.uinodes.clear();
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

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_material_nodes<M: UiMaterial>(
    extracted_uinodes: Res<ExtractedUiMaterialNodes<M>>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
    ui_material_pipeline: Res<UiMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_materials: Res<RenderAssets<PreparedUiMaterial<M>>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(&ExtractedView, &Msaa)>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_function = draw_functions.read().id::<DrawUiMaterial<M>>();

    for (entity, extracted_uinode) in extracted_uinodes.uinodes.iter() {
        let Ok((view, msaa)) = views.get_mut(extracted_uinode.camera_entity) else {
            continue;
        };

        let Some(transparent_phase) =
            transparent_render_phases.get_mut(&extracted_uinode.camera_entity)
        else {
            continue;
        };

        let Some(material) = render_materials.get(extracted_uinode.material) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_material_pipeline,
            UiMaterialKey {
                hdr: view.hdr,
                bind_group_data: material.key.clone(),
                sample_count: msaa.samples() as i8,
            },
        );
        transparent_phase
            .items
            .reserve(extracted_uinodes.uinodes.len());
        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: (*entity, MainEntity::from(*entity)),
            sort_key: (
                FloatOrd(extracted_uinode.stack_index as f32 + stack_z_offsets::MATERIAL),
                entity.index(),
            ),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::maybe_dynamic_offset(None),
        });
    }
}
