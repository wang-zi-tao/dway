use core::f32;
use std::{hash::Hash, marker::PhantomData, ops::Deref};

use bevy::{
    core_pipeline::{
        core_2d::graph::Node2d,
        tonemapping::{get_lut_bindings, DebandDither, Tonemapping, TonemappingLuts},
    },
    ecs::{
        entity::EntityHashMap,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            *,
        },
    },
    image::{ImageSampler, TextureFormatPixelInfo},
    math::{Affine3, FloatOrd},
    mesh::MeshVertexBufferLayoutRef,
    post_process::msaa_writeback::MsaaWritebackNode,
    render::{
        batching::{
            no_gpu_preprocessing::{
                batch_and_prepare_sorted_render_phase, clear_batched_cpu_instance_buffers,
                write_batched_instance_buffer, BatchedInstanceBuffer,
            },
            GetBatchData, NoAutomaticBatching,
        },
        camera::extract_cameras,
        globals::{GlobalsBuffer, GlobalsUniform},
        mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo},
        render_asset::{RenderAssetPlugin, RenderAssets},
        render_graph::{RenderGraphExt, ViewNodeRunner},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity, TemporaryRenderEntity},
        texture::{DefaultImageSampler, FallbackImage, GpuImage},
        view::*,
        Extract, RenderApp, RenderSet, RenderStartup,
    },
    shader::ShaderRef,
    sprite_render::{
        init_mesh_2d_pipeline, tonemapping_pipeline_key, Material2d, Material2dBindGroupId,
        Material2dKey, Material2dPipeline, Mesh2dPipeline, Mesh2dPipelineKey, MeshFlags,
        PreparedMaterial2d,
    },
    ui_render::{
        graph::{NodeUi, SubGraphUi},
        TransparentUi, UiCameraMap, UiCameraView,
    },
};

use self::graph::NodeUiExt;
use super::UiRenderOffset;
use crate::prelude::*;

pub mod graph {
    use bevy::render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum NodeUiExt {
        MsaaWriteback,
    }
}

#[derive(Default, Clone, Component, Debug, Reflect, PartialEq, Eq, Deref, DerefMut)]
#[reflect(Component)]
#[require(UiMeshTransform, Node, Mesh2d)]
pub struct UiMesh(Handle<Mesh>);

#[derive(Component, Deref, DerefMut, Debug, Clone, Reflect)]
pub struct UiMeshTransform(Transform);
impl Default for UiMeshTransform {
    fn default() -> Self {
        Self(Transform::default().with_scale(Vec3::new(1.0, -1.0, 1.0)))
    }
}
impl From<Transform> for UiMeshTransform {
    fn from(transform: Transform) -> Self {
        Self(transform)
    }
}
impl UiMeshTransform {
    pub fn new(transform: Transform) -> Self {
        Self(transform)
    }

    pub fn new_ui_transform() -> Self {
        Self(Transform::default())
    }
}

impl From<Handle<Mesh>> for UiMesh {
    fn from(handle: Handle<Mesh>) -> Self {
        Self(handle)
    }
}

#[derive(Default)]
pub struct UiMeshPlugin;
impl Plugin for UiMeshPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiMeshTransform>()
            .register_type::<UiMesh>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderUiMesh2dInstances>()
                .init_resource::<SpecializedMeshPipelines<UiMesh2dPipeline>>()
                .add_systems(
                    RenderStartup,
                    init_ui_mesh_2d_pipeline.after(init_mesh_2d_pipeline),
                )
                .add_systems(
                    ExtractSchedule,
                    extract_ui_mesh_node.chain().after(extract_cameras),
                )
                .add_systems(
                    bevy::render::Render,
                    (
                        batch_and_prepare_sorted_render_phase::<TransparentUi, UiMesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<UiMesh2dPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                        prepare_mesh2d_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh2d_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                        clear_batched_cpu_instance_buffers::<UiMesh2dPipeline>
                            .in_set(RenderSet::Cleanup)
                            .after(RenderSet::Render),
                    ),
                )
                .add_render_graph_node::<ViewNodeRunner<MsaaWritebackNode>>(
                    SubGraphUi,
                    NodeUiExt::MsaaWriteback,
                )
                .add_render_graph_edge(
                    SubGraphUi,
                    Node2d::EndMainPassPostProcessing,
                    NodeUiExt::MsaaWriteback,
                )
                .add_render_graph_edge(
                    SubGraphUi,
                    Node2d::EndMainPassPostProcessing,
                    NodeUiExt::MsaaWriteback,
                )
                .add_render_graph_edge(SubGraphUi, NodeUiExt::MsaaWriteback, NodeUi::UiPass);
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let render_device = render_app.world().resource::<RenderDevice>();
            let batched_instance_buffer =
                BatchedInstanceBuffer::<UiMesh2dUniform>::new(render_device);
            render_app.insert_resource(batched_instance_buffer);
        }
    }
}

pub struct UiMeshMaterialPlugin<R: Material2d>(PhantomData<R>);
impl<R: Material2d> Default for UiMeshMaterialPlugin<R> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<T: Material2d> Plugin for UiMeshMaterialPlugin<T>
where
    T::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.world().contains_resource::<Assets<T>>();
        if !app.is_plugin_added::<RenderAssetPlugin<PreparedMaterial2d<T>>>() {
            app.add_plugins(RenderAssetPlugin::<PreparedMaterial2d<T>>::default());
        }
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMesh<T>>()
                .init_resource::<RenderUiMeshMaterialInstances<T>>()
                .init_resource::<SpecializedMeshPipelines<UiMeshMaterialPipeline<T>>>()
                .add_systems(
                    RenderStartup,
                    init_ui_mesh_material_pipeline::<T>.after(init_ui_mesh_2d_pipeline),
                )
                .add_systems(ExtractSchedule, extract_ui_mesh_handle::<T>)
                .add_systems(
                    bevy::render::Render,
                    queue_ui_meshes::<T>.in_set(RenderSet::QueueMeshes),
                );
        }
    }
}

#[derive(Default, Clone, Reflect, ShaderType)]
pub struct UiMeshNodeUniform {
    pub position: Vec2,
    pub size: Vec2,
}

#[derive(Resource, Deref, DerefMut)]
pub struct RenderUiMeshMaterialInstances<M: Material2d>(EntityHashMap<AssetId<M>>);

impl<M: Material2d> Default for RenderUiMeshMaterialInstances<M> {
    fn default() -> Self {
        Self(Default::default())
    }
}

fn extract_ui_mesh_handle<M: Material2d>(
    mut material_instances: ResMut<RenderUiMeshMaterialInstances<M>>,
    query: Extract<Query<(Entity, &ViewVisibility, &MeshMaterial2d<M>), With<UiMesh>>>,
) {
    material_instances.clear();
    for (main_entity, view_visibility, handle) in &query {
        if view_visibility.get() {
            material_instances.insert(main_entity, handle.0.id());
        }
    }
}

pub fn extract_ui_mesh_node(
    mut commands: Commands,
    mut render_mesh_instances: ResMut<RenderUiMesh2dInstances>,
    query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &InheritedVisibility,
            &UiMeshTransform,
            &GlobalTransform,
            &UiMesh,
            Option<&UiRenderOffset>,
            &ComputedUiTargetCamera,
            Has<NoAutomaticBatching>,
            Option<&CalculatedClip>,
        )>,
    >,
    view_query: Query<&ExtractedView>,
    default_ui_camera: Extract<DefaultUiCamera>,
    render_entity_lookup: Extract<Query<RenderEntity>>,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();

    render_mesh_instances.clear();
    for (
        main_entity,
        computed_node,
        view_visibility,
        mesh_transform,
        transform,
        handle,
        zoffset,
        camera,
        no_automatic_batching,
        clip,
    ) in query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(&camera) else {
            continue;
        };

        let rect = Rect::from_center_size(transform.translation().xy(), computed_node.size());
        let clip_rect = clip.map(|clip| clip.clip).unwrap_or(rect).intersect(rect);
        let clip_offset = clip_rect.center() - rect.center();

        let Ok(extracted_view) = view_query.get(extracted_camera_entity) else {
            continue;
        };
        let viewport_size = extracted_view.viewport.zw().as_vec2();
        let zoffset = zoffset.map(|z| z.0).unwrap_or_default();

        if clip_rect.width() > 0.0 && clip_rect.height() > 0.0 {
            render_mesh_instances.insert(
                commands.spawn(TemporaryRenderEntity).id(),
                RenderUiMeshInstance {
                    transforms: Mesh2dTransforms {
                        transform: (&GlobalTransform::default()
                            .mul_transform(
                                Transform::from_scale(
                                    ((viewport_size) / clip_rect.size()).extend(1.0),
                                )
                                .with_translation(viewport_size.extend(0.0) * 0.5),
                            )
                            .mul_transform(Transform::from_translation(-clip_offset.extend(0.0)))
                            .mul_transform(**mesh_transform)
                            .affine())
                            .into(),
                        flags: MeshFlags::empty().bits(),
                        rect: clip_rect,
                    },
                    mesh_asset_id: handle.0.id(),
                    material_bind_group_id: Material2dBindGroupId::default(),
                    automatic_batching: !no_automatic_batching,
                    stack_index: computed_node.stack_index(),
                    extracted_camera_entity,
                    main_entity,
                    zoffset,
                },
            );
        }
    }
}

#[derive(Resource, Clone)]
pub struct UiMesh2dPipeline {
    pub node_layout: BindGroupLayout,
    pub mesh2d_pipeline: Mesh2dPipeline,
}

impl Deref for UiMesh2dPipeline {
    type Target = Mesh2dPipeline;

    fn deref(&self) -> &Self::Target {
        &self.mesh2d_pipeline
    }
}

pub fn init_ui_mesh_2d_pipeline(
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
    mut commands: Commands,
) {
    let node_layout = render_device.create_bind_group_layout(
        "ui_mesh2d_node_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            GpuArrayBuffer::<UiMeshNodeUniform>::binding_layout(&render_device),
        ),
    );

    commands.insert_resource(UiMesh2dPipeline {
        node_layout,
        mesh2d_pipeline: mesh2d_pipeline.clone(),
    })
}

impl UiMesh2dPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<GpuImage>,
        handle_option: &Option<Handle<Image>>,
    ) -> Option<(&'a TextureView, &'a Sampler)> {
        self.mesh2d_pipeline
            .get_image_texture(gpu_images, handle_option)
    }
}

impl GetBatchData for UiMesh2dPipeline {
    type BufferData = UiMesh2dUniform;
    type CompareData = (Material2dBindGroupId, AssetId<Mesh>);
    type Param = SRes<RenderUiMesh2dInstances>;

    fn get_batch_data(
        mesh_instances: &SystemParamItem<Self::Param>,
        (entity, _): (Entity, MainEntity),
    ) -> Option<(Self::BufferData, Option<Self::CompareData>)> {
        let mesh_instance = mesh_instances.get(&entity)?;
        Some((
            (&mesh_instance.transforms).into(),
            mesh_instance.automatic_batching.then_some((
                mesh_instance.material_bind_group_id,
                mesh_instance.mesh_asset_id,
            )),
        ))
    }
}

#[derive(Component)]
pub struct Mesh2dTransforms {
    pub transform: Affine3,
    pub rect: Rect,
    pub flags: u32,
}

#[derive(ShaderType, Clone)]
pub struct UiMesh2dUniform {
    // Affine 4x3 matrix transposed to 3x4
    pub transform: [Vec4; 3],
    // 3x3 matrix packed in mat2x4 and f32 as:
    //   [0].xyz, [1].x,
    //   [1].yz, [2].xy
    //   [2].z
    pub inverse_transpose_model_a: [Vec4; 2],
    pub inverse_transpose_model_b: f32,
    pub flags: u32,
}

impl From<&Mesh2dTransforms> for UiMesh2dUniform {
    fn from(mesh_transforms: &Mesh2dTransforms) -> Self {
        let (inverse_transpose_model_a, inverse_transpose_model_b) =
            mesh_transforms.transform.inverse_transpose_3x3();
        Self {
            transform: mesh_transforms.transform.to_transpose(),
            inverse_transpose_model_a,
            inverse_transpose_model_b,
            flags: mesh_transforms.flags,
        }
    }
}

impl SpecializedMeshPipeline for UiMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let spec = self.mesh2d_pipeline.specialize(key, layout)?;
        Ok(RenderPipelineDescriptor {
            layout: vec![
                self.view_layout.clone(),
                self.mesh_layout.clone(),
                self.node_layout.clone(),
            ],
            label: Some("ui_transparent_mesh2d_pipeline".into()),
            ..spec
        })
    }
}

#[derive(Resource)]
pub struct UiMeshMaterialPipeline<M: Material2d> {
    pub mesh2d_pipeline: UiMesh2dPipeline,
    pub material2d_layout: BindGroupLayout,
    pub vertex_shader: Option<Handle<Shader>>,
    pub fragment_shader: Option<Handle<Shader>>,
    marker: PhantomData<M>,
}

impl<M: Material2d> Clone for UiMeshMaterialPipeline<M> {
    fn clone(&self) -> Self {
        Self {
            mesh2d_pipeline: self.mesh2d_pipeline.clone(),
            material2d_layout: self.material2d_layout.clone(),
            vertex_shader: self.vertex_shader.clone(),
            fragment_shader: self.fragment_shader.clone(),
            marker: PhantomData,
        }
    }
}

impl<M: Material2d> SpecializedMeshPipeline for UiMeshMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = Material2dKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key.mesh_key, layout)?;
        if let Some(vertex_shader) = &self.vertex_shader {
            descriptor.vertex.shader = vertex_shader.clone();
        }

        if let Some(fragment_shader) = &self.fragment_shader {
            descriptor.fragment.as_mut().unwrap().shader = fragment_shader.clone();
        }
        descriptor.layout = vec![
            self.mesh2d_pipeline.view_layout.clone(),
            self.mesh2d_pipeline.mesh_layout.clone(),
            self.material2d_layout.clone(),
        ];

        M::specialize(&mut descriptor, layout, key)?;
        Ok(descriptor)
    }
}

pub fn init_ui_mesh_material_pipeline<M: Material2d>(
    asset_server: Res<AssetServer>,
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<UiMesh2dPipeline>,
    mut commands: Commands,
) {
    let material2d_layout = M::bind_group_layout(&render_device);

    commands.insert_resource(UiMeshMaterialPipeline {
        mesh2d_pipeline: mesh2d_pipeline.clone(),
        material2d_layout,
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
        marker: PhantomData::<M>,
    })
}

#[derive(Resource)]
pub struct UiMesh2dBindGroup {
    pub value: BindGroup,
}

pub fn prepare_mesh2d_bind_group(
    mut commands: Commands,
    mesh2d_pipeline: Res<UiMesh2dPipeline>,
    render_device: Res<RenderDevice>,
    mesh2d_uniforms: Res<BatchedInstanceBuffer<UiMesh2dUniform>>,
) {
    if let Some(binding) = mesh2d_uniforms.binding() {
        commands.insert_resource(UiMesh2dBindGroup {
            value: render_device.create_bind_group(
                "ui_mesh2d_bind_group",
                &mesh2d_pipeline.mesh_layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[derive(Component)]
pub struct UiMesh2dViewBindGroup {
    pub value: BindGroup,
}

pub fn prepare_mesh2d_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh2d_pipeline: Res<UiMesh2dPipeline>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &Tonemapping), (With<ExtractedView>, With<Camera2d>)>,
    globals_buffer: Res<GlobalsBuffer>,
    images: Res<RenderAssets<GpuImage>>,
    fallback_image: Res<FallbackImage>,
    tonemapping_luts: Res<TonemappingLuts>,
) {
    if let (Some(view_binding), Some(globals)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        for (entity, tonemapping) in &views {
            let lut_bindings =
                get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
            let view_bind_group = render_device.create_bind_group(
                "ui_mesh2d_view_bind_group",
                &mesh2d_pipeline.view_layout,
                &BindGroupEntries::sequential((
                    view_binding.clone(),
                    globals.clone(),
                    lut_bindings.0,
                    lut_bindings.1,
                )),
            );

            commands.entity(entity).insert(UiMesh2dViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

pub struct RenderUiMeshInstance {
    pub stack_index: u32,
    pub transforms: Mesh2dTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: Material2dBindGroupId,
    pub automatic_batching: bool,
    pub extracted_camera_entity: Entity,
    pub main_entity: Entity,
    pub zoffset: f32,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderUiMesh2dInstances(pub EntityHashMap<RenderUiMeshInstance>);

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_meshes<M: Material2d>(
    transparent_draw_functions: Res<DrawFunctions<TransparentUi>>,
    material2d_pipeline: Res<UiMeshMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<UiMeshMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<RenderAssets<PreparedMaterial2d<M>>>,
    mut render_mesh_instances: ResMut<RenderUiMesh2dInstances>,
    render_material_instances: Res<RenderUiMeshMaterialInstances<M>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(&ExtractedView, Option<&Tonemapping>, Option<&DebandDither>)>,
    mut render_views: Query<&UiCameraView, With<ExtractedView>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (index, (entity, mesh_instance)) in render_mesh_instances.iter_mut().enumerate() {
        let Some(material_asset_id) = render_material_instances.get(&mesh_instance.main_entity)
        else {
            debug!(entity =?mesh_instance.main_entity, "material is not prepared");
            continue;
        };
        let Some(material2d) = render_materials.get(*material_asset_id) else {
            debug!(entity =?mesh_instance.main_entity, "material is not prepared");
            continue;
        };
        let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
            debug!(mesh=?mesh_instance.mesh_asset_id,"mesh is not prepared");
            continue;
        };

        let Ok(default_camera_view) = render_views.get_mut(mesh_instance.extracted_camera_entity)
        else {
            continue;
        };

        let Ok((view, tonemapping, dither)) = views.get_mut(default_camera_view.0) else {
            debug!(entity =?mesh_instance.extracted_camera_entity ,"camera is not valid");
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawUiMesh<M>>();

        let mut view_key = Mesh2dPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }

        let mesh_key =
            view_key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology());

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &material2d_pipeline,
            Material2dKey {
                mesh_key,
                bind_group_data: material2d.key.clone(),
            },
            &mesh.layout,
        );

        let pipeline_id = match pipeline_id {
            Ok(id) => id,
            Err(err) => {
                error!("{}", err);
                continue;
            }
        };

        mesh_instance.material_bind_group_id = material2d.get_bind_group_id();

        transparent_phase.add(TransparentUi {
            sort_key: FloatOrd(mesh_instance.stack_index as f32 + mesh_instance.zoffset),
            entity: (*entity, MainEntity::from(mesh_instance.main_entity)),
            pipeline: pipeline_id,
            draw_function: draw_transparent_pbr,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: true,
        });
    }
}

type DrawUiMesh<M> = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetMesh2dBindGroup<1>,
    SetUiMeshBindGroup<M, 2>,
    DoDrawUiMesh,
);

pub struct SetMesh2dViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dViewBindGroup<I> {
    type ItemQuery = ();
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<UiMesh2dViewBindGroup>);

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, mesh2d_view_bind_group): ROQueryItem<'w, '_, Self::ViewQuery>,
        _view: std::option::Option<()>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &mesh2d_view_bind_group.value, &[view_uniform.offset]);

        RenderCommandResult::Success
    }
}

pub struct SetMesh2dBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMesh2dBindGroup<I> {
    type ItemQuery = ();
    type Param = SRes<UiMesh2dBindGroup>;
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: std::option::Option<()>,
        mesh2d_bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mut dynamic_offsets: [u32; 1] = Default::default();
        let mut offset_count = 0;
        if let PhaseItemExtraIndex::DynamicOffset(dynamic_offset) = item.extra_index() {
            dynamic_offsets[offset_count] = dynamic_offset;
            offset_count += 1;
        }
        pass.set_bind_group(
            I,
            &mesh2d_bind_group.into_inner().value,
            &dynamic_offsets[..offset_count],
        );
        RenderCommandResult::Success
    }
}

pub struct SetUiMeshBindGroup<M: Material2d, const I: usize>(PhantomData<M>);
impl<P: PhaseItem, M: Material2d, const I: usize> RenderCommand<P> for SetUiMeshBindGroup<M, I> {
    type ItemQuery = ();
    type Param = (
        SRes<RenderAssets<PreparedMaterial2d<M>>>,
        SRes<RenderUiMeshMaterialInstances<M>>,
    );
    type ViewQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: std::option::Option<()>,
        (materials, material_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let materials = materials.into_inner();
        let material_instances = material_instances.into_inner();
        let material_instance = material_instances.get(&*item.main_entity()).unwrap();
        let Some(material2d) = materials.get(*material_instance) else {
            debug!(material=?material_instance,"material is not prepared");
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &material2d.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DoDrawUiMesh;
impl<P: PhaseItem> RenderCommand<P> for DoDrawUiMesh {
    type ItemQuery = ();
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderUiMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = &'static ExtractedView;

    #[inline]
    fn render<'w>(
        item: &P,
        view: &ExtractedView,
        _item_query: std::option::Option<()>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let RenderUiMeshInstance {
            mesh_asset_id,
            transforms,
            ..
        } = render_mesh2d_instances.get(&item.entity()).unwrap();
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            debug!("mesh is not prepared");
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            debug!("vertex buffer is not prepared");
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));

        let viewport = view.viewport;
        let rect = transforms.rect.intersect(Rect::new(
            viewport.x as f32,
            viewport.y as f32,
            (viewport.x + viewport.z) as f32,
            (viewport.y + viewport.w) as f32,
        ));
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return RenderCommandResult::Success;
        }
        pass.set_viewport(
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height(),
            0.0,
            1.0,
        );

        let batch_range = item.batch_range();
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    debug!("index buffer is not prepared");
                    return RenderCommandResult::Skip;
                };

                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), 0, *index_format);

                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    batch_range.clone(),
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, batch_range.clone());
            }
        }

        let viewport = view.viewport;
        pass.set_viewport(
            viewport.x as f32,
            viewport.y as f32,
            viewport.z as f32,
            viewport.w as f32,
            0.0,
            1.0,
        );

        RenderCommandResult::Success
    }
}

pub struct PrepareNextFrameMaterials<M: Material2d> {
    assets: Vec<(AssetId<M>, M)>,
}

impl<M: Material2d> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}
