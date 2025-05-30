use core::f32;
use std::{hash::Hash, marker::PhantomData};

use bevy::{
    core_pipeline::{
        core_2d::graph::Node2d,
        msaa_writeback::MsaaWritebackNode,
        tonemapping::{DebandDither, Tonemapping},
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
        mesh::{
            allocator::MeshAllocator, MeshVertexBufferLayoutRef, RenderMesh, RenderMeshBufferInfo,
        },
        render_asset::{RenderAssetPlugin, RenderAssets},
        render_graph::{RenderGraphApp, ViewNodeRunner},
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        sync_world::{MainEntity, RenderEntity, TemporaryRenderEntity},
        texture::{DefaultImageSampler, GpuImage},
        view::*,
        Extract, RenderApp, RenderSet,
    },
    sprite::{
        tonemapping_pipeline_key, Material2d, Material2dBindGroupId, Material2dKey,
        Material2dPipeline, Mesh2dPipelineKey, MeshFlags, PreparedMaterial2d, MESH2D_SHADER_HANDLE,
    },
    ui::{
        graph::{NodeUi, SubGraphUi},
        TransparentUi,
    },
};

use self::graph::NodeUiExt;
use super::UiRenderOffset;
use crate::{make_bundle, prelude::*};

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
pub struct UiMeshHandle(Handle<Mesh>);

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

impl From<Handle<Mesh>> for UiMeshHandle {
    fn from(handle: Handle<Mesh>) -> Self {
        Self(handle)
    }
}

#[derive(Default)]
pub struct UiMeshPlugin;
impl Plugin for UiMeshPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiMeshTransform>()
            .register_type::<UiMeshHandle>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderUiMesh2dInstances>()
                .init_resource::<SpecializedMeshPipelines<UiMesh2dPipeline>>()
                .add_systems(
                    ExtractSchedule,
                    (apply_deferred, extract_ui_mesh_node)
                        .chain()
                        .after(extract_cameras),
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
            render_app
                .insert_resource(batched_instance_buffer)
                .init_resource::<UiMesh2dPipeline>();
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
                .add_systems(ExtractSchedule, extract_ui_mesh_handle::<T>)
                .add_systems(
                    bevy::render::Render,
                    queue_ui_meshes::<T>.in_set(RenderSet::QueueMeshes),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiMeshMaterialPipeline<T>>();
            render_app.init_resource::<Material2dPipeline<T>>();
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
    query: Extract<Query<(Entity, &ViewVisibility, &MeshMaterial2d<M>), With<UiMeshHandle>>>,
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
            &ViewVisibility,
            &UiMeshTransform,
            &GlobalTransform,
            &UiMeshHandle,
            Option<&UiRenderOffset>,
            Option<&TargetCamera>,
            Has<NoAutomaticBatching>,
            Option<&CalculatedClip>,
        )>,
    >,
    view_query: Query<&ExtractedView>,
    default_ui_camera: Extract<DefaultUiCamera>,
    render_entity_lookup: Extract<Query<RenderEntity>>,
) {
    render_mesh_instances.clear();
    for (
        main_entity,
        computed_node,
        view_visibility,
        mesh_transform,
        transform,
        handle,
        zoffset,
        target_camera,
        no_automatic_batching,
        clip,
    ) in query.iter()
    {
        if !view_visibility.get() {
            continue;
        }

        let Some(camera_entity) = target_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
        else {
            continue;
        };
        let Ok(camera_entity) = render_entity_lookup.get(camera_entity) else {
            continue;
        };

        let rect = Rect::from_center_size(transform.translation().xy(), computed_node.size());
        let clip_rect = clip.map(|clip| clip.clip).unwrap_or(rect).intersect(rect);
        let clip_offset = clip_rect.center() - rect.center();

        let Ok(extracted_view) = view_query.get(camera_entity) else {
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
                    camera: camera_entity,
                    main_entity,
                    zoffset,
                },
            );
        }
    }
}

#[derive(Resource, Clone)]
pub struct UiMesh2dPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub node_layout: BindGroupLayout,
    // This dummy white texture is to be used in place of optional textures
    pub dummy_white_gpu_image: GpuImage,
    pub per_object_buffer_batch_size: Option<u32>,
}

impl FromWorld for UiMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<RenderQueue>,
            Res<DefaultImageSampler>,
        )> = SystemState::new(world);
        let (render_device, render_queue, default_sampler) = system_state.get_mut(world);
        let render_device = render_device.into_inner();
        let view_layout = render_device.create_bind_group_layout(
            "ui_mesh2d_view_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let node_layout = render_device.create_bind_group_layout(
            "ui_mesh2d_node_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                GpuArrayBuffer::<UiMeshNodeUniform>::binding_layout(render_device),
            ),
        );

        let mesh_layout = render_device.create_bind_group_layout(
            "ui_mesh2d_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                GpuArrayBuffer::<UiMesh2dUniform>::binding_layout(render_device),
            ),
        );
        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.size(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };
        UiMesh2dPipeline {
            view_layout,
            mesh_layout,
            node_layout,
            dummy_white_gpu_image,
            per_object_buffer_batch_size: GpuArrayBuffer::<UiMesh2dUniform>::batch_size(
                render_device,
            ),
        }
    }
}

impl UiMesh2dPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<GpuImage>,
        handle_option: &Option<Handle<Image>>,
    ) -> Option<(&'a TextureView, &'a Sampler)> {
        if let Some(handle) = handle_option {
            let gpu_image = gpu_images.get(handle)?;
            Some((&gpu_image.texture_view, &gpu_image.sampler))
        } else {
            Some((
                &self.dummy_white_gpu_image.texture_view,
                &self.dummy_white_gpu_image.sampler,
            ))
        }
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
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        if layout.0.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        if layout.0.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(4));
        }

        if key.msaa_samples() > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }

        if key.contains(Mesh2dPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(Mesh2dPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            match method {
                Mesh2dPipelineKey::TONEMAP_METHOD_NONE => {
                    shader_defs.push("TONEMAP_METHOD_NONE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_ACES_FITTED => {
                    shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_AGX => {
                    shader_defs.push("TONEMAP_METHOD_AGX".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM => {
                    shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC => {
                    shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE => {
                    shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
                }
                _ => {}
            }
            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(Mesh2dPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        let vertex_buffer_layout = layout.0.get_layout(&vertex_attributes)?;

        let format = match key.contains(Mesh2dPipelineKey::HDR) {
            true => ViewTarget::TEXTURE_FORMAT_HDR,
            false => TextureFormat::bevy_default(),
        };
        let mut push_constant_ranges = Vec::with_capacity(1);
        if cfg!(all(
            feature = "webgl",
            target_arch = "wasm32",
            not(feature = "webgpu")
        )) {
            push_constant_ranges.push(PushConstantRange {
                stages: ShaderStages::VERTEX,
                range: 0..4,
            });
        }

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: MESH2D_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: MESH2D_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.mesh_layout.clone()],
            push_constant_ranges,
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("ui_transparent_mesh2d_pipeline".into()),
            zero_initialize_workgroup_memory: false,
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

impl<M: Material2d> FromWorld for UiMeshMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let render_device = world.resource::<RenderDevice>();
        let material2d_layout = M::bind_group_layout(render_device);

        UiMeshMaterialPipeline {
            mesh2d_pipeline: world.resource::<UiMesh2dPipeline>().clone(),
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
            marker: PhantomData,
        }
    }
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
    views: Query<Entity, With<ExtractedView>>,
    globals_buffer: Res<GlobalsBuffer>,
) {
    if let (Some(view_binding), Some(globals)) = (
        view_uniforms.uniforms.binding(),
        globals_buffer.buffer.binding(),
    ) {
        for entity in &views {
            let view_bind_group = render_device.create_bind_group(
                "ui_mesh2d_view_bind_group",
                &mesh2d_pipeline.view_layout,
                &BindGroupEntries::sequential((view_binding.clone(), globals.clone())),
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
    pub camera: Entity,
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
    mut views: Query<(
        &ExtractedView,
        &Msaa,
        Option<&Tonemapping>,
        Option<&DebandDither>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (entity, mesh_instance) in render_mesh_instances.iter_mut() {
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

        let Ok((view, msaa, tonemapping, dither)) = views.get_mut(mesh_instance.camera) else {
            debug!(entity =?mesh_instance.camera ,"camera is not valid");
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&mesh_instance.camera)
        else {
            continue;
        };

        let draw_transparent_pbr = transparent_draw_functions.read().id::<DrawUiMesh<M>>();

        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

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
            sort_key: (
                FloatOrd(mesh_instance.stack_index as f32 + mesh_instance.zoffset),
                entity.index(),
            ),
            entity: (*entity, MainEntity::from(mesh_instance.main_entity)),
            pipeline: pipeline_id,
            draw_function: draw_transparent_pbr,
            batch_range: 0..1,
            extra_index: PhaseItemExtraIndex::NONE,
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
        (view_uniform, mesh2d_view_bind_group): ROQueryItem<'w, Self::ViewQuery>,
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
        if let Some(dynamic_offset) = item.extra_index().as_dynamic_offset() {
            dynamic_offsets[offset_count] = dynamic_offset.get();
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

make_bundle! {
    @material2d
    UiMeshBundle {
        pub mesh: UiMeshHandle,
        pub mesh_transform: UiMeshTransform,
    }
}
