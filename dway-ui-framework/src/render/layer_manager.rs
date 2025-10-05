use bevy::{
    asset::load_internal_asset,
    ecs::{
        component::{ComponentId, HookContext},
        entity::EntityHashSet,
        system::EntityCommand,
        world::DeferredWorld,
    },
    math::FloatOrd,
    platform::collections::HashMap,
    render::{
        camera::{ImageRenderTarget, NormalizedRenderTarget, RenderTarget},
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        render_resource::{
            encase::internal::{BufferMut, Writer},
            AsBindGroupError, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
    },
    transform::components::Transform,
    ui::{ui_focus_system, UiSystem},
    window::{PrimaryWindow, WindowRef},
};
use bevy_relationship::reexport::Entity;

use crate::{
    prelude::*,
    shader::{
        fill::Fill,
        shape::{RoundedRect, Shape},
        BindGroupBuilder, BindGroupLayoutBuilder, BuildBindGroup, ShaderAsset, ShaderBuilder,
        ShaderPlugin, ShaderVariables, ShapeRender, UniformLayout,
    },
    widgets::util::visibility,
    UiFrameworkSystems,
};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Debug)]
pub enum LayerKind {
    #[default]
    Normal,
    Blur,
    Canvas,
}

#[derive(Component, SmartDefault, Reflect)]
#[require(UiTargetCamera=UiTargetCamera(Entity::PLACEHOLDER))]
pub struct RenderToLayer {
    pub layer_manager: Option<Entity>,
    pub layer_camera: Option<Entity>,
    pub ui_background: Handle<Image>,
    #[default(Vec2::ONE)]
    pub background_size: Vec2,
    pub layer_kind: LayerKind,
}
impl RenderToLayer {
    pub fn blur() -> Self {
        Self {
            layer_kind: LayerKind::Blur,
            ..Default::default()
        }
    }
}

structstruck::strike! {
    #[derive(Component, Reflect, Debug)]
    pub struct SetWindowTarget ( Option<
            #[derive(Reflect, Debug)]
            struct BackupRenderTargetInner {
                window_target: RenderTarget,
                layer: RenderTarget,
        }> )
}

#[derive(Component, Reflect, Debug)]
pub struct LayerCamera {
    layer_manager: Entity,
    layer_kind: LayerKind,
}

impl LayerCamera {
    pub fn layer_manager(&self) -> Entity {
        self.layer_manager
    }

    pub fn layer_kind(&self) -> LayerKind {
        self.layer_kind
    }
}

#[derive(Debug, Reflect, Clone)]
pub(crate) struct BaseLayerRef {
    pub(crate) camera: Entity,
    surface: Handle<Image>,
}
impl BaseLayerRef {
    fn placeholder() -> Self {
        BaseLayerRef {
            camera: Entity::PLACEHOLDER,
            surface: default(),
        }
    }

    fn update_camera(
        &self,
        camera_query: &mut Query<(&mut Camera, &mut SetWindowTarget)>,
        render_target: &mut Option<RenderTarget>,
        window_target: &RenderTarget,
    ) {
        let (mut camera, mut backup) = camera_query.get_mut(self.camera).unwrap();
        if let Some(render_target) = render_target.take() {
            camera.target = render_target.clone();
            backup.0 = None;
        } else {
            camera.target = RenderTarget::Image(ImageRenderTarget {
                handle: self.surface.clone(),
                scale_factor: FloatOrd(1.0),
            });
            backup.0 = Some(BackupRenderTargetInner {
                window_target: window_target.clone(),
                layer: camera.target.clone(),
            });
        }
    }
}

pub trait Layer {
    fn new(
        world: &mut DeferredWorld,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self;

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>);

    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut Query<(&mut Camera, &mut SetWindowTarget)>,
        render_target: &mut Option<RenderTarget>,
        window_target: &RenderTarget,
    );

    fn update_background(
        &mut self,
        enable: bool,
        background_query: &mut Query<(&mut ImageNode, &mut Visibility)>,
        background: &mut Handle<Image>,
    );

    fn update_rects(&mut self, rects: &[Rect], meshes: &mut Assets<Mesh>);
}

#[derive(Debug, Reflect, Clone)]
pub(crate) struct LayerRef {
    pub(crate) camera: Entity,
    pub(crate) background_entity: Entity,
    pub(crate) background_image: Handle<Image>,
    pub(crate) surface: Handle<Image>,
    pub(crate) area: Handle<Mesh>,
}

impl LayerRef {
    fn placeholder() -> Self {
        LayerRef {
            camera: Entity::PLACEHOLDER,
            background_entity: Entity::PLACEHOLDER,
            background_image: default(),
            surface: default(),
            area: default(),
        }
    }
}

impl Layer for LayerRef {
    fn new(
        world: &mut DeferredWorld,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self {
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let size = UVec2::ONE;
        let surface = create_image(size, &mut image_assets);
        let transform = Transform::from_xyz(0.0, 0.0, order as f32 * 8192.0);
        let camera_entity = world
            .commands()
            .spawn((
                Camera2d::default(),
                Camera {
                    clear_color: Color::BLACK.into(),
                    target: render_target.clone(),
                    order,
                    ..Default::default()
                },
                transform,
                LayerCamera {
                    layer_manager: manager_entity,
                    layer_kind: LayerKind::Blur,
                },
                SetWindowTarget(None),
            ))
            .set_parent(manager_entity)
            .id();
        let background_entity = world
            .commands()
            .spawn((
                ImageNode::default(),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Node::default()
                },
                UiTargetCamera(camera_entity),
            ))
            .id();
        LayerRef {
            camera: camera_entity,
            background_entity,
            surface,
            area: Default::default(),
            background_image: Default::default(),
        }
    }

    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut Query<(&mut Camera, &mut SetWindowTarget)>,
        render_target: &mut Option<RenderTarget>,
        window_target: &RenderTarget,
    ) {
        let (mut camera, mut backup) = camera_query.get_mut(self.camera).unwrap();
        if enable {
            if let Some(render_target) = render_target.take() {
                backup.0 = Some(BackupRenderTargetInner {
                    window_target: window_target.clone(),
                    layer: render_target.clone(),
                });
                camera.target = render_target;
            } else {
                camera.target = RenderTarget::Image(ImageRenderTarget {
                    handle: self.surface.clone(),
                    scale_factor: FloatOrd(1.0),
                });
                backup.0 = None;
            }
        } else {
            camera.target = RenderTarget::Image(ImageRenderTarget {
                handle: self.surface.clone(),
                scale_factor: FloatOrd(1.0),
            });
            backup.0 = None;
        }
        camera.is_active = enable;
    }

    fn update_background(
        &mut self,
        enable: bool,
        background_query: &mut Query<(&mut ImageNode, &mut Visibility)>,
        background: &mut Handle<Image>,
    ) {
        if enable {
            let (mut image, mut vis) = background_query.get_mut(self.background_entity).unwrap();
            *vis = visibility(enable);
            image.image = background.clone();
            self.background_image = background.clone();
            *background = self.surface.clone();
        }
    }

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>) {
        update_image(size, &mut self.surface, images);
    }

    fn update_rects(&mut self, rects: &[Rect], meshes: &mut Assets<Mesh>) {
        let mut positions: Vec<Vec3> = vec![];
        let mut indices: Vec<u32> = vec![];
        for rect in rects {
            positions.extend([
                Vec3::new(rect.max.x, 0.0, rect.min.y),
                Vec3::new(rect.min.x, 0.0, rect.min.y),
                Vec3::new(rect.min.x, 0.0, rect.max.y),
                Vec3::new(rect.max.x, 0.0, rect.max.y),
            ]);
            indices.extend([0, 1, 2, 0, 2, 3]);
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U32(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        self.area = meshes.add(mesh);
    }
}

structstruck::strike! {
    #[derive(Debug, Reflect, Clone)]
    pub(crate) struct BlurLayer{
        pub(crate) blur_method:
            #[derive(Clone, Copy, Reflect, Debug, PartialEq)]
            pub enum BlurMethod {
                Kawase{ layer: usize, radius: f32 },
                Dual{ layer: usize, radius: f32 },
                // Gaussian{ kernel_size: usize},
            },
        pub(crate) blur_image: Handle<Image>,
        pub(crate) layer: LayerRef,
        pub(crate) shader: Handle<Shader>,
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
pub enum BlurMethodKind {
    Kawase,
    Dual,
}

impl BlurMethod {
    pub fn kind(&self) -> BlurMethodKind {
        match self {
            BlurMethod::Kawase { .. } => BlurMethodKind::Kawase,
            BlurMethod::Dual { .. } => BlurMethodKind::Dual,
        }
    }

    pub fn kawase() -> Self {
        Self::Kawase {
            layer: 4,
            radius: 1.0,
        }
    }

    pub fn dual() -> Self {
        Self::Dual {
            layer: 4,
            radius: 1.0,
        }
    }
}

impl Layer for BlurLayer {
    fn new(
        world: &mut DeferredWorld,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self {
        let inner = LayerRef::new(world, order, render_target, manager_entity);
        let size = UVec2::ONE;
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let blur_image = create_image(size, &mut image_assets);
        Self {
            blur_method: BlurMethod::dual(),
            layer: inner,
            shader: Default::default(),
            blur_image,
        }
    }

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>) {
        update_image(size, &mut self.blur_image, images);
        self.layer.update_size(size, images)
    }

    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut Query<(&mut Camera, &mut SetWindowTarget)>,
        render_target: &mut Option<RenderTarget>,
        window_target: &RenderTarget,
    ) {
        self.layer
            .update_camera(enable, camera_query, render_target, window_target)
    }

    fn update_background(
        &mut self,
        enable: bool,
        background_query: &mut Query<(&mut ImageNode, &mut Visibility)>,
        background: &mut Handle<Image>,
    ) {
        self.layer
            .update_background(enable, background_query, background);
    }

    fn update_rects(&mut self, rects: &[Rect], meshes: &mut Assets<Mesh>) {
        self.layer.update_rects(rects, meshes)
    }
}

const KWASE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(48163214082082095667413131907815359807);

const DUAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(318570688117048185338870793837454854575);

impl BlurLayer {
    pub fn update_shader(&mut self) {
        let shader_handle = match self.blur_method {
            BlurMethod::Kawase { .. } => KWASE_SHADER_HANDLE,
            BlurMethod::Dual { .. } => DUAL_SHADER_HANDLE,
        };
        self.shader = shader_handle;
    }

    fn placeholder() -> Self {
        BlurLayer {
            blur_method: BlurMethod::dual(),
            blur_image: default(),
            layer: LayerRef::placeholder(),
            shader: default(),
        }
    }
}

#[derive(Component, Debug, Reflect, Clone)]
#[require(SetWindowTarget=SetWindowTarget(None), Camera)]
#[component(on_insert=on_insert_layer_manager)]
#[component(on_replace=on_replace_layer_manager)]
pub struct LayerManager {
    pub(crate) base_layer: BaseLayerRef,
    pub(crate) canvas_layer: LayerRef,
    pub(crate) blur_layer: BlurLayer,
    pub(crate) size: UVec2,
    pub(crate) canvas_enable: bool,
    pub(crate) blur_enable: bool,
    pub(crate) window_target: RenderTarget,
    pub(crate) render_target: RenderTarget,
}

fn create_image_descripteor(size: UVec2) -> Image {
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
                ..default()
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        },
        ..Default::default()
    };
    image.resize(Extent3d {
        width: size.x,
        height: size.y,
        depth_or_array_layers: 1,
        ..default()
    });
    image
}

fn update_image(size: UVec2, handle: &mut Handle<Image>, images: &mut Assets<Image>) {
    let image = create_image_descripteor(size);
    if handle.is_strong() {
        images.insert(handle.id(), image);
    } else {
        *handle = images.add(image);
    }
}

fn create_image(size: UVec2, images: &mut Assets<Image>) -> Handle<Image> {
    let image = create_image_descripteor(size);
    images.add(image)
}

impl Default for LayerManager {
    fn default() -> Self {
        Self {
            base_layer: BaseLayerRef::placeholder(),
            canvas_layer: LayerRef::placeholder(),
            blur_layer: BlurLayer::placeholder(),
            size: UVec2::ONE,
            canvas_enable: Default::default(),
            blur_enable: Default::default(),
            window_target: Default::default(),
            render_target: Default::default(),
        }
    }
}

impl LayerManager {
    pub fn with_render_target(mut self, render_target: RenderTarget) -> Self {
        self.render_target = render_target;
        self
    }

    pub fn with_window_target(mut self, entity: Entity) -> Self {
        self.window_target = RenderTarget::Window(WindowRef::Entity(entity));
        self
    }

    pub fn get_ui_background(&self, kind: LayerKind) -> Handle<Image> {
        match kind {
            LayerKind::Normal => Default::default(),
            LayerKind::Blur => self.blur_layer.blur_image.clone(),
            LayerKind::Canvas => self.canvas_layer.background_image.clone(),
        }
    }

    pub fn get_camera(&self, kind: LayerKind) -> Entity {
        match kind {
            LayerKind::Normal => self.base_layer.camera,
            LayerKind::Blur => self.blur_layer.layer.camera,
            LayerKind::Canvas => self.canvas_layer.camera,
        }
    }
}

fn on_insert_layer_manager(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let camera = world.get::<Camera>(entity).unwrap();
    let render_target = camera.target.clone();

    let canvas_layer = LayerRef::new(&mut world, 10, &render_target, entity);
    let blur_layer = BlurLayer::new(&mut world, 20, &render_target, entity);

    let mut layer_manager = world.get_mut::<LayerManager>(entity).unwrap();
    layer_manager.base_layer.camera = entity;
    layer_manager.canvas_layer = canvas_layer;
    layer_manager.blur_layer = blur_layer;
    layer_manager.render_target = render_target;
}

fn on_replace_layer_manager(mut world: DeferredWorld, context: HookContext) {
    let entity = context.entity;
    let layer_manager = std::mem::take(&mut *world.get_mut::<LayerManager>(entity).unwrap());

    let mut commands = world.commands();

    let mut despawn = |entity: Entity| {
        if let Ok(mut c) = commands.get_entity(entity) {
            c.despawn();
        }
    };
    despawn(layer_manager.base_layer.camera);
    despawn(layer_manager.canvas_layer.camera);
    despawn(layer_manager.canvas_layer.background_entity);
    despawn(layer_manager.blur_layer.layer.camera);
    despawn(layer_manager.blur_layer.layer.background_entity);

    if let Some(mut camera) = world.get_mut::<Camera>(entity) {
        camera.target = layer_manager.render_target;
    }
}

pub fn update_ui_root(
    mut query: Query<(
        Entity,
        &mut RenderToLayer,
        &mut UiTargetCamera,
        &mut ComputedNodeTarget,
        &ChildOf,
    )>,
    layer_manager_query: Query<Ref<LayerManager>>,
    layer_camera_query: Query<Ref<LayerCamera>>,
    mut commands: Commands,
    mut update_next_frame: Local<EntityHashSet>,
) {
    for (entity, mut render_to_layer, mut target_camera, mut node_target, parent) in &mut query {
        let (layer_manager_entity, layer_manager) = {
            let Some(target_camera_entity) = render_to_layer
                .layer_manager
                .or_else(|| node_target.camera())
            else {
                continue;
            };
            if let Ok(layer_manager) = layer_manager_query.get(target_camera_entity) {
                (target_camera_entity, layer_manager)
            } else if let Ok(layer_camera) = layer_camera_query.get(target_camera_entity) {
                (
                    layer_camera.layer_manager,
                    layer_manager_query.get(layer_camera.layer_manager).unwrap(),
                )
            } else {
                update_next_frame.insert(entity);
                continue;
            }
        };

        if !update_next_frame.remove(&entity)
            && !render_to_layer.is_changed()
            && !layer_manager.is_changed()
        {
            continue;
        }

        render_to_layer.layer_manager = Some(layer_manager_entity);
        let layer_camera = layer_manager.get_camera(render_to_layer.layer_kind);
        render_to_layer.layer_camera = Some(layer_camera);

        render_to_layer.background_size = layer_manager.size.as_vec2();
        render_to_layer.ui_background = layer_manager.get_ui_background(render_to_layer.layer_kind);

        target_camera.0 = layer_camera;
        *node_target = default();

        commands.entity(entity).insert(parent.clone());
    }
}

pub fn update_layers(
    mut layer_manager_query: Query<(Entity, &mut LayerManager)>,
    mut camera_query: Query<(&mut Camera, &mut SetWindowTarget)>,
    mut background_query: Query<(&mut ImageNode, &mut Visibility)>,
    window_query: Query<&Window>,
    ui_root_query: Query<
        (
            &ViewVisibility,
            &RenderToLayer,
            &ComputedNode,
            &GlobalTransform,
        ),
        With<RenderToLayer>,
    >,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let primary_window = primary_window.iter().next();
    let mut layer_rects = HashMap::<Entity, Vec<Rect>>::new();
    for (visibility, render_to_layer, computed_node, global_transform) in &ui_root_query {
        if !**visibility
            || render_to_layer.layer_kind == LayerKind::Normal
            || render_to_layer.layer_camera.is_none()
        {
            continue;
        }
        let Some(layer_camera) = render_to_layer.layer_camera else {
            continue;
        };
        let rects = layer_rects.entry(layer_camera).or_insert(vec![]);
        let mut rect =
            Rect::from_center_size(global_transform.translation().xy(), computed_node.size());
        loop {
            let mut changed = false;
            for (i, r) in rects.iter().enumerate() {
                let union = r.union(rect);
                if union.width() * union.height()
                    < r.width() * r.height() + rect.width() * rect.height()
                {
                    rects.swap_remove(i);
                    rect = union;
                    changed = true;
                    break;
                }
            }
            if !changed {
                rects.push(rect);
                break;
            }
        }
    }

    for (entity, mut layer_manager) in &mut layer_manager_query {
        let mut image_size_changed = false;
        {
            let (camera, _) = camera_query.get(entity).unwrap();

            let size = match camera.target.normalize(primary_window) {
                Some(NormalizedRenderTarget::Window(window_ref)) => window_query
                    .get(window_ref.entity())
                    .map(|w| UVec2::new(w.physical_width(), w.physical_height()))
                    .unwrap_or(UVec2::ONE),
                Some(NormalizedRenderTarget::Image(image)) => images
                    .get(image.handle.id())
                    .map(Image::size)
                    .unwrap_or(UVec2::ONE),
                _ => UVec2::ONE,
            };
            if layer_manager.size != size {
                image_size_changed = true;
                layer_manager.size = size;
            }
        }

        let mut layer_changed = false;
        {
            let blur_enable = layer_rects.contains_key(&layer_manager.blur_layer.layer.camera);
            let canvas_enable = layer_rects.contains_key(&layer_manager.canvas_layer.camera);
            if blur_enable != layer_manager.blur_enable
                || canvas_enable != layer_manager.canvas_enable
            {
                layer_manager.canvas_enable = canvas_enable;
                layer_manager.blur_enable = blur_enable;
                layer_changed = true;
            }
        }

        if layer_changed && layer_manager.blur_enable && layer_manager.is_changed() {
            layer_manager.blur_layer.update_shader();
        }

        if !(image_size_changed || layer_changed || layer_manager.is_changed()) {
            continue;
        }

        if image_size_changed {
            update_image(
                layer_manager.size,
                &mut layer_manager.base_layer.surface,
                &mut images,
            );
            let size = layer_manager.size;
            layer_manager.canvas_layer.update_size(size, &mut images);
            layer_manager.blur_layer.update_size(size, &mut images);
        }

        {
            let mut render_target = Some(layer_manager.render_target.clone());
            layer_manager.blur_layer.layer.update_camera(
                layer_manager.blur_enable,
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
            );
            layer_manager.canvas_layer.update_camera(
                layer_manager.canvas_enable,
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
            );
            layer_manager.base_layer.update_camera(
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
            );
        }

        {
            let canvas_enable = layer_manager.canvas_enable;
            let blur_enable = layer_manager.blur_enable;
            let mut background = layer_manager.base_layer.surface.clone();
            layer_manager.canvas_layer.update_background(
                canvas_enable,
                &mut background_query,
                &mut background,
            );
            layer_manager.blur_layer.update_background(
                blur_enable,
                &mut background_query,
                &mut background,
            );
        }

        {
            if layer_manager.blur_enable {
                if let Some(rects) = layer_rects.get(&layer_manager.blur_layer.layer.camera) {
                    layer_manager
                        .blur_layer
                        .layer
                        .update_rects(rects, &mut meshes);
                }
            }
            if layer_manager.canvas_enable {
                if let Some(rects) = layer_rects.get(&layer_manager.canvas_layer.camera) {
                    layer_manager.canvas_layer.update_rects(rects, &mut meshes);
                }
            }
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct FillWithLayer {
    pub texture: Handle<Image>,
    pub texture_size: Vec2,
}

impl FillWithLayer {
    pub fn new(texture: Handle<Image>, texture_size: Vec2) -> Self {
        Self {
            texture,
            texture_size,
        }
    }
}
impl Fill for FillWithLayer {
    fn to_wgsl(builder: &mut ShaderBuilder, _var: &ShaderVariables) -> String {
        let var_image_texture = builder.get_binding("background_texture", "", "texture_2d<f32>");
        let var_image_sampler = builder.get_binding("background_sampler", "", "sampler");
        let uniform_size = builder.get_uniform("background_size", "", "vec2<f32>");
        format!("textureSample({var_image_texture}, {var_image_sampler}, in.position.xy/{uniform_size})")
    }
}
impl BuildBindGroup for FillWithLayer {
    fn bind_group_layout_entries(builder: &mut BindGroupLayoutBuilder) {
        builder.add_image();
    }

    fn unprepared_bind_group(
        &self,
        builder: &mut BindGroupBuilder,
    ) -> Result<(), AsBindGroupError> {
        builder.add_image(&self.texture)?;
        Ok(())
    }

    fn update_layout(&self, layout: &mut UniformLayout) {
        layout.update_layout(&self.texture_size);
    }

    fn write_uniform<B: BufferMut>(&self, layout: &mut UniformLayout, writer: &mut Writer<B>) {
        layout.write_uniform(&self.texture_size, writer);
    }
}

pub type FillWithLayerMaterial = ShapeRender<RoundedRect, FillWithLayer>;

pub fn update_ui_material(
    mut query: Query<
        (
            &RenderToLayer,
            &mut MaterialNode<ShaderAsset<FillWithLayerMaterial>>,
        ),
        Changed<RenderToLayer>,
    >,
    mut material_assets: ResMut<Assets<ShaderAsset<FillWithLayerMaterial>>>,
) {
    for (render_to_layer, mut shader_handle) in &mut query {
        let material = RoundedRect::new(16.0).with_effect(FillWithLayer {
            texture: render_to_layer.ui_background.clone(),
            texture_size: render_to_layer.background_size,
        });
        shader_handle.0 = material_assets.add(material);
    }
}

pub fn before_ui_focus_system(mut query: Query<(&SetWindowTarget, &mut Camera)>) {
    for (backup, mut camera) in &mut query {
        if let Some(backup) = backup.0.as_ref() {
            camera.target = backup.window_target.clone();
        }
    }
}

pub fn after_ui_focus_system(mut query: Query<(&SetWindowTarget, &mut Camera)>) {
    for (backup, mut camera) in &mut query {
        if let Some(backup) = backup.0.as_ref() {
            camera.target = backup.layer.clone();
        }
    }
}

pub struct LayerManagerPlugin;
impl Plugin for LayerManagerPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            KWASE_SHADER_HANDLE,
            "blur/kawase.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, DUAL_SHADER_HANDLE, "blur/dual.wgsl", Shader::from_wgsl);
        app.register_type::<LayerManager>()
            .register_type::<LayerCamera>()
            .register_type::<RenderToLayer>()
            .add_plugins(ShaderPlugin::<FillWithLayerMaterial>::default())
            .add_systems(
                PreUpdate,
                (
                    before_ui_focus_system.before(ui_focus_system),
                    after_ui_focus_system.after(ui_focus_system),
                )
                    .in_set(UiSystem::Focus),
            )
            .add_systems(
                Last,
                (
                    update_layers.in_set(UiFrameworkSystems::UpdateLayers),
                    update_ui_root.in_set(UiFrameworkSystems::UpdateLayers),
                    update_ui_material.in_set(UiFrameworkSystems::UpdateLayersMaterial),
                )
                    .chain(),
            );
    }
}
