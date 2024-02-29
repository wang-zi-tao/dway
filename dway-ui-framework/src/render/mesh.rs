use crate::prelude::*;
use bevy::{
    app::{App, Plugin},
    core_pipeline::tonemapping::{DebandDither, Tonemapping},
    ecs::{
        entity::EntityHashMap, prelude::{Entity, EventReader}, query::{ROQueryItem, With}, schedule::IntoSystemConfigs, system::lifetimeless::{Read, SRes}, system::*, world::{FromWorld, World}
    },
    render::{
        batching::{
            batch_and_prepare_render_phase, write_batched_instance_buffer, GetBatchData,
            NoAutomaticBatching,
        },
        globals::{GlobalsBuffer, GlobalsUniform},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::{prepare_assets, RenderAssets},
        render_phase::{AddRenderCommand, *},
        render_resource::{binding_types::uniform_buffer, *},
        renderer::{RenderDevice, RenderQueue},
        texture::{
            BevyDefault, DefaultImageSampler, FallbackImage, GpuImage, Image, ImageSampler,
            TextureFormatPixelInfo,
        },
        view::*,
        Extract, ExtractSchedule, RenderApp, RenderSet,
    },
    sprite::{
        tonemapping_pipeline_key, Material2d, Material2dBindGroupId, Material2dKey,
        Mesh2dPipelineKey, Mesh2dTransforms, Mesh2dUniform, MeshFlags, PreparedMaterial2d,
        MESH2D_SHADER_HANDLE,
    },
    transform::prelude::GlobalTransform,
    ui::{TransparentUi, UiStack},
    utils::{FloatOrd, HashMap, HashSet},
};
use std::{hash::Hash, marker::PhantomData};

#[derive(Default, Clone, Component, Debug, Reflect, PartialEq, Eq, Deref, DerefMut)]
#[reflect(Component)]
pub struct UiMeshHandle(Handle<Mesh>);

#[derive(Component, Deref, DerefMut, Debug, Clone)]
pub struct UiMeshTransform(Transform);
impl Default for UiMeshTransform {
    fn default() -> Self {
        Self(Transform::default().with_scale(Vec3::new(1.0, -1.0, 1.0)))
    }
}
impl From<Transform> for UiMeshTransform{
    fn from(transform: Transform) -> Self{
        Self(transform)
    }
}
impl UiMeshTransform{
    pub fn new(transform: Transform) -> Self{
        Self(transform)
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
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<RenderUiMesh2dInstances>()
                .init_resource::<SpecializedMeshPipelines<UiMesh2dPipeline>>()
                .add_systems(ExtractSchedule, extract_ui_mesh_node)
                .add_systems(
                    bevy::render::Render,
                    (
                        batch_and_prepare_render_phase::<TransparentUi, UiMesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<UiMesh2dPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                        prepare_mesh2d_bind_group.in_set(RenderSet::PrepareBindGroups),
                        prepare_mesh2d_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let buffer =
                GpuArrayBuffer::<Mesh2dUniform>::new(render_app.world.resource::<RenderDevice>());
            render_app
                .insert_resource(buffer)
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
        app.init_asset::<T>();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiMesh<T>>()
                .init_resource::<ExtractedUiMeshMaterial<T>>()
                .init_resource::<RenderUiMesh<T>>()
                .init_resource::<RenderUiMeshMaterialInstances<T>>()
                .init_resource::<SpecializedMeshPipelines<UiMeshMaterialPipeline<T>>>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_ui_mesh_material_asset::<T>,
                        extract_ui_mesh_handle::<T>,
                    ),
                )
                .add_systems(
                    bevy::render::Render,
                    (
                        prepare_ui_mesh::<T>
                            .in_set(RenderSet::PrepareAssets)
                            .after(prepare_assets::<Image>),
                        queue_ui_meshes::<T>
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_ui_mesh::<T>),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiMeshMaterialPipeline<T>>();
        }
    }
}

pub fn extract_ui_mesh_material_asset<M: Material2d>(
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
            AssetEvent::LoadedWithDependencies { .. } => {}
        }
    }

    let mut extracted_assets = Vec::new();
    for id in changed_assets.drain() {
        if let Some(asset) = assets.get(id) {
            extracted_assets.push((id, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedUiMeshMaterial {
        extracted: extracted_assets,
        removed,
    });
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
    query: Extract<Query<(Entity, &ViewVisibility, &Handle<M>), With<UiMeshHandle>>>,
) {
    material_instances.clear();
    for (entity, view_visibility, handle) in &query {
        if view_visibility.get() {
            material_instances.insert(entity, handle.id());
        }
    }
}

#[derive(Component)]
pub struct UiMesh;

pub fn extract_ui_mesh_node(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut render_mesh_instances: ResMut<RenderUiMesh2dInstances>,
    ui_stack: Extract<Res<UiStack>>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &UiMeshTransform,
            &GlobalTransform,
            &UiMeshHandle,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    render_mesh_instances.clear();
    let mut entities = Vec::with_capacity(*previous_len);

    for (stack_index, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok((entity, view_visibility, mesh_transform, transform, handle, no_automatic_batching)) =
            query.get(*entity)
        {
            if !view_visibility.get() {
                continue;
            }
            // FIXME: Remove this - it is just a workaround to enable rendering to work as
            // render commands require an entity to exist at the moment.
            entities.push((entity, UiMesh));
            render_mesh_instances.insert(
                entity,
                RenderUiMeshInstance {
                    transforms: Mesh2dTransforms {
                        transform: (&transform.mul_transform(**mesh_transform).affine()).into(),
                        flags: MeshFlags::empty().bits(),
                    },
                    mesh_asset_id: handle.0.id(),
                    material_bind_group_id: Material2dBindGroupId::default(),
                    automatic_batching: !no_automatic_batching,
                    stack_index,
                },
            );
        }
    }
    *previous_len = entities.len();
    commands.insert_or_spawn_batch(entities);
}

#[derive(Resource, Clone)]
pub struct UiMesh2dPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
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
            "mesh2d_view_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    // View
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<GlobalsUniform>(false),
                ),
            ),
        );

        let mesh_layout = render_device.create_bind_group_layout(
            "mesh2d_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                GpuArrayBuffer::<Mesh2dUniform>::binding_layout(render_device),
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
                size: image.size_f32(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };
        UiMesh2dPipeline {
            view_layout,
            mesh_layout,
            dummy_white_gpu_image,
            per_object_buffer_batch_size: GpuArrayBuffer::<Mesh2dUniform>::batch_size(
                render_device,
            ),
        }
    }
}

impl UiMesh2dPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<Image>,
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
    type Param = SRes<RenderUiMesh2dInstances>;
    type CompareData = (Material2dBindGroupId, AssetId<Mesh>);
    type BufferData = Mesh2dUniform;

    fn get_batch_data(
        mesh_instances: &SystemParamItem<Self::Param>,
        entity: Entity,
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

impl SpecializedMeshPipeline for UiMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        let mut vertex_attributes = Vec::new();

        if layout.contains(Mesh::ATTRIBUTE_POSITION) {
            shader_defs.push("VERTEX_POSITIONS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_POSITION.at_shader_location(0));
        }

        if layout.contains(Mesh::ATTRIBUTE_NORMAL) {
            shader_defs.push("VERTEX_NORMALS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_NORMAL.at_shader_location(1));
        }

        if layout.contains(Mesh::ATTRIBUTE_UV_0) {
            shader_defs.push("VERTEX_UVS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_UV_0.at_shader_location(2));
        }

        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push("VERTEX_TANGENTS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        if layout.contains(Mesh::ATTRIBUTE_COLOR) {
            shader_defs.push("VERTEX_COLORS".into());
            vertex_attributes.push(Mesh::ATTRIBUTE_COLOR.at_shader_location(4));
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

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

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
            label: Some("transparent_mesh2d_pipeline".into()),
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
        layout: &MeshVertexBufferLayout,
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
        descriptor.multisample.count = 1;

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
    mesh2d_uniforms: Res<GpuArrayBuffer<Mesh2dUniform>>,
) {
    if let Some(binding) = mesh2d_uniforms.binding() {
        commands.insert_resource(UiMesh2dBindGroup {
            value: render_device.create_bind_group(
                "mesh2d_bind_group",
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
                "mesh2d_view_bind_group",
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
    pub stack_index: usize,
    pub transforms: Mesh2dTransforms,
    pub mesh_asset_id: AssetId<Mesh>,
    pub material_bind_group_id: Material2dBindGroupId,
    pub automatic_batching: bool,
}

#[derive(Default, Resource, Deref, DerefMut)]
pub struct RenderUiMesh2dInstances(pub EntityHashMap<RenderUiMeshInstance>);

#[allow(clippy::too_many_arguments)]
pub fn queue_ui_meshes<M: Material2d>(
    transparent_draw_functions: Res<DrawFunctions<TransparentUi>>,
    material2d_pipeline: Res<UiMeshMaterialPipeline<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<UiMeshMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderUiMesh<M>>,
    mut render_mesh_instances: ResMut<RenderUiMesh2dInstances>,
    render_material_instances: Res<RenderUiMeshMaterialInstances<M>>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        &mut RenderPhase<TransparentUi>,
    )>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    if render_material_instances.is_empty() {
        return;
    }

    for (view, visible_entities, tonemapping, dither, mut transparent_phase) in &mut views {
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
        for visible_entity in &visible_entities.entities {
            let Some(material_asset_id) = render_material_instances.get(visible_entity) else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.get_mut(visible_entity) else {
                continue;
            };
            let Some(material2d) = render_materials.get(material_asset_id) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };
            let mesh_key =
                view_key | Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);

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
                    FloatOrd(mesh_instance.stack_index as f32),
                    visible_entity.index(),
                ),
                entity: *visible_entity,
                pipeline: pipeline_id,
                draw_function: draw_transparent_pbr,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
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
    type Param = ();
    type ViewQuery = (Read<ViewUniformOffset>, Read<UiMesh2dViewBindGroup>);
    type ItemQuery = ();

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
    type Param = SRes<UiMesh2dBindGroup>;
    type ViewQuery = ();
    type ItemQuery = ();

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
        if let Some(dynamic_offset) = item.dynamic_offset() {
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
    type Param = (
        SRes<RenderUiMesh<M>>,
        SRes<RenderUiMeshMaterialInstances<M>>,
    );
    type ViewQuery = ();
    type ItemQuery = ();

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
        let Some(material_instance) = material_instances.get(&item.entity()) else {
            return RenderCommandResult::Failure;
        };
        let Some(material2d) = materials.get(material_instance) else {
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, &material2d.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DoDrawUiMesh;
impl<P: PhaseItem> RenderCommand<P> for DoDrawUiMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SRes<RenderUiMesh2dInstances>);
    type ViewQuery = ();
    type ItemQuery = ();

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        _item_query: std::option::Option<()>,
        (meshes, render_mesh2d_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();

        let Some(RenderUiMeshInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.entity())
        else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));

        let batch_range = item.batch_range();
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        pass.set_push_constants(
            ShaderStages::VERTEX,
            0,
            &(batch_range.start as i32).to_le_bytes(),
        );
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, batch_range.clone());
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, batch_range.clone());
            }
        }
        RenderCommandResult::Success
    }
}
// type RenderUiMesh<T: Material2d> = bevy::sprite::RenderMaterials2d<T>;
/// Stores all prepared representations of [`Material2d`] assets for as long as they exist.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderUiMesh<T: Material2d>(HashMap<AssetId<T>, PreparedMaterial2d<T>>);

impl<T: Material2d> Default for RenderUiMesh<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// All [`Material2d`] values of a given type that should be prepared next frame.
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

#[derive(Resource)]
pub struct ExtractedUiMeshMaterial<M: Material2d> {
    extracted: Vec<(AssetId<M>, M)>,
    removed: Vec<AssetId<M>>,
}

impl<M: Material2d> Default for ExtractedUiMeshMaterial<M> {
    fn default() -> Self {
        Self {
            extracted: Default::default(),
            removed: Default::default(),
        }
    }
}

pub fn prepare_ui_mesh<M: Material2d>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_assets: ResMut<ExtractedUiMeshMaterial<M>>,
    mut render_materials: ResMut<RenderUiMesh<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<UiMeshMaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (id, material) in queued_assets {
        match prepare_material2d(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((id, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_assets.removed) {
        render_materials.remove(&removed);
    }

    for (asset_id, material) in std::mem::take(&mut extracted_assets.extracted) {
        match prepare_material2d(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                render_materials.insert(asset_id, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((asset_id, material));
            }
        }
    }
}

fn prepare_material2d<M: Material2d>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &FallbackImage,
    pipeline: &UiMeshMaterialPipeline<M>,
) -> Result<PreparedMaterial2d<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material2d_layout,
        render_device,
        images,
        fallback_image,
    )?;
    Ok(PreparedMaterial2d {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
        depth_bias: material.depth_bias(),
    })
}

/// A component bundle for entities with a [`Mesh2dHandle`] and a [`Material2d`].
#[derive(Bundle, Clone)]
pub struct UiMeshBundle<M: Material2d> {
    pub mesh: UiMeshHandle,
    pub mesh_transform: UiMeshTransform,
    pub material: Handle<M>,
    /// Describes the logical size of the node
    pub node: Node,
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This component is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This component is automatically updated by the [`TransformPropagate`](`bevy_transform::TransformSystem::TransformPropagate`) systems.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}

impl<M: Material2d> Default for UiMeshBundle<M> {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            node: Default::default(),
            style: Default::default(),
            focus_policy: Default::default(),
            z_index: Default::default(),
            mesh_transform: Default::default(),
        }
    }
}
