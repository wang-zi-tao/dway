use bevy::{
    asset::load_internal_asset,
    ecs::{
        component::{ComponentId, HookContext},
        entity::EntityHashSet,
        query::QueryData,
        system::{EntityCommand, SystemParam},
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

#[derive(Component, Reflect)]
#[require(LayerRenderArea)]
#[require(UiTargetCamera=UiTargetCamera(Entity::PLACEHOLDER))]
#[component(on_insert=on_insert_layer)]
pub struct RenderToLayer {
    layer_manager: Entity,
    layer_camera: Option<Entity>,
    layer_kind: LayerKind,
}

impl RenderToLayer {
    pub fn new(camera: Entity, kind: LayerKind) -> Self {
        Self {
            layer_manager: camera,
            layer_camera: None,
            layer_kind: kind,
        }
    }

    pub fn layer_kind(&self) -> LayerKind {
        self.layer_kind
    }

    pub fn layer_manager(&self) -> Entity {
        self.layer_manager
    }

    pub fn layer_camera(&self) -> Option<Entity> {
        self.layer_camera
    }
}

#[derive(Component, Reflect, Default)]
pub struct LayerRenderArea;

fn on_insert_layer(mut world: DeferredWorld, context: HookContext) {
    let Some(layer) = world.get::<RenderToLayer>(context.entity) else {
        error!("RenderToLayer component not found on entity {:?}", context.entity);
        return
    };

    let layer_manager_entity = layer.layer_manager();
    let kind = layer.layer_kind();

    let layer_manager = world.get::<LayerManager>(layer_manager_entity).unwrap();

    let camera_entity = layer_manager.get_camera(kind);

    let mut layer = world.get_mut::<RenderToLayer>(context.entity).unwrap();

    layer.layer_camera = Some(camera_entity);

    let mut target_camera = world.get_mut::<UiTargetCamera>(context.entity).unwrap();
    target_camera.0 = camera_entity;
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
    ui_background: Handle<Image>,
    pub background_size: Vec2,
}

impl LayerCamera {
    pub fn layer_manager(&self) -> Entity {
        self.layer_manager
    }

    pub fn layer_kind(&self) -> LayerKind {
        self.layer_kind
    }

    pub fn ui_background(&self) -> &Handle<Image> {
        &self.ui_background
    }
}

#[derive(SystemParam)]
pub struct CameraQuery<'w, 's> {
    pub camera: Query<
        'w,
        's,
        (
            &'static mut Camera,
            &'static mut SetWindowTarget,
            Option<&'static mut LayerCamera>,
        ),
    >,
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

    fn new(world: &mut DeferredWorld, manager_entity: Entity, label: &'static str) -> Self {
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let size = UVec2::ONE;
        let surface = create_image(size, &mut image_assets, label);
        BaseLayerRef {
            camera: manager_entity,
            surface: surface,
        }
    }

    fn update_camera(
        &self,
        camera_query: &mut CameraQuery,
        render_target: &mut RenderTarget,
        window_target: &RenderTarget,
    ) {
        let (mut camera, mut backup, _) = camera_query.camera.get_mut(self.camera).unwrap();
        backup.0 = Some(BackupRenderTargetInner {
            window_target: window_target.clone(),
            layer: render_target.clone(),
        });
        camera.target = render_target.clone();
    }
}

pub trait Layer {
    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>, label: &'static str);

    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut CameraQuery,
        render_target: &mut RenderTarget,
        window_target: &RenderTarget,
        size: Vec2,
    );

    fn update_rects(&mut self, rects: &[Rect], surface_size: UVec2, meshes: &mut Assets<Mesh>);
}

#[derive(Debug, Reflect, Clone)]
pub(crate) struct LayerRef {
    pub(crate) camera: Entity,
    pub(crate) background_entity: Entity,
    pub(crate) background_image: Handle<Image>,
}

impl LayerRef {
    fn placeholder() -> Self {
        LayerRef {
            camera: Entity::PLACEHOLDER,
            background_entity: Entity::PLACEHOLDER,
            background_image: default(),
        }
    }
}

impl LayerRef {
    fn new(
        world: &mut DeferredWorld,
        order: isize,
        render_target: &RenderTarget,
        ui_background: Handle<Image>,
        manager_entity: Entity,
        background_image_label: &'static str,
    ) -> Self {
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let size = UVec2::ONE;
        let background_image = create_image(size, &mut image_assets, background_image_label);
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
                    ui_background,
                    background_size: Default::default(),
                },
                SetWindowTarget(None),
            ))
            .set_parent(manager_entity)
            .id();
        let background_entity = world
            .commands()
            .spawn((
                ImageNode::new(background_image.clone()),
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
            background_image,
        }
    }
}

impl Layer for LayerRef {
    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut CameraQuery,
        render_target: &mut RenderTarget,
        window_target: &RenderTarget,
        size: Vec2,
    ) {
        let (mut camera, mut backup, mut layer) = camera_query.camera.get_mut(self.camera).unwrap();
        if let Some(mut layer) = layer {
            layer.background_size = size;
        }
        if enable {
            backup.0 = Some(BackupRenderTargetInner {
                window_target: window_target.clone(),
                layer: render_target.clone(),
            });
            camera.target = render_target.clone();
            *render_target = RenderTarget::Image(ImageRenderTarget {
                handle: self.background_image.clone(),
                scale_factor: FloatOrd(1.0),
            });
        } else {
            backup.0 = None;
        }
        camera.is_active = enable;
    }

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>, label: &'static str) {
        update_image(size, &mut self.background_image, images, label);
    }

    fn update_rects(&mut self, _rects: &[Rect], _surface_size: UVec2, _meshes: &mut Assets<Mesh>) {
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
        pub(crate) area: Handle<Mesh>,
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

    pub fn padding_size(&self) -> f32 {
        match self {
            BlurMethod::Kawase { radius, .. } => *radius * 2.0,
            BlurMethod::Dual { radius, .. } => *radius * 2.0,
        }
    }
}

impl BlurLayer {
    fn new(
        world: &mut DeferredWorld,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self {
        let size = UVec2::ONE;
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let blur_image = create_image(
            size,
            &mut image_assets,
            "layer_manager/blur_layer/blur_image",
        );
        let inner = LayerRef::new(
            world,
            order,
            render_target,
            blur_image.clone(),
            manager_entity,
            "layer_manager/blur_layer/background_image",
        );

        Self {
            blur_method: BlurMethod::dual(),
            layer: inner,
            shader: Default::default(),
            blur_image,
            area: Default::default(),
        }
    }
}

impl Layer for BlurLayer {
    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>, label: &'static str) {
        update_image(
            size,
            &mut self.blur_image,
            images,
            "layer_manager/blur_layer/blur_image",
        );
        self.layer.update_size(size, images, label);
    }

    fn update_camera(
        &self,
        enable: bool,
        camera_query: &mut CameraQuery,
        render_target: &mut RenderTarget,
        window_target: &RenderTarget,
        size: Vec2,
    ) {
        self.layer
            .update_camera(enable, camera_query, render_target, window_target, size)
    }

    fn update_rects(&mut self, rects: &[Rect], surface_size: UVec2, meshes: &mut Assets<Mesh>) {
        self.layer.update_rects(rects, surface_size, meshes);

        let mut positions: Vec<Vec2> = Vec::with_capacity(rects.len() * 4);
        let mut indices: Vec<u16> = Vec::with_capacity(rects.len() * 6);

        let paddings = self.blur_method.padding_size();
        let rects = merge_rects(rects.iter().map(|rect| Rect {
            min: rect.min - Vec2::splat(paddings),
            max: rect.max + Vec2::splat(paddings),
        }));

        for rect in rects {
            let surface_size = surface_size.as_vec2();
            let offset = positions.len() as u16;
            positions.extend([
                Vec2::new(rect.max.x, rect.min.y) / surface_size,
                Vec2::new(rect.min.x, rect.min.y) / surface_size,
                Vec2::new(rect.min.x, rect.max.y) / surface_size,
                Vec2::new(rect.max.x, rect.max.y) / surface_size,
            ]);
            indices.extend([
                offset + 0,
                offset + 1,
                offset + 2,
                offset + 0,
                offset + 2,
                offset + 3,
            ]);
        }
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(Indices::U16(indices))
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, positions);
        self.area = meshes.add(mesh);
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
            area: Default::default(),
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

fn create_image_descripteor(size: UVec2, label: &'static str) -> Image {
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some(label),
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
                | TextureUsages::COPY_SRC
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

fn update_image(
    size: UVec2,
    handle: &mut Handle<Image>,
    images: &mut Assets<Image>,
    label: &'static str,
) {
    let image = create_image_descripteor(size, label);
    if handle.is_strong() {
        images.insert(handle.id(), image);
    } else {
        *handle = images.add(image);
    }
}

fn create_image(size: UVec2, images: &mut Assets<Image>, label: &'static str) -> Handle<Image> {
    let image = create_image_descripteor(size, label);
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

    let base_layer = BaseLayerRef::new(&mut world, entity, "layer_manager/base_layer/surface");
    let canvas_layer = LayerRef::new(
        &mut world,
        10,
        &render_target,
        default(),
        entity,
        "layer_manager/canvas_layer/background_image",
    );
    let blur_layer = BlurLayer::new(&mut world, 20, &render_target, entity);

    let mut layer_manager = world.get_mut::<LayerManager>(entity).unwrap();
    layer_manager.base_layer = base_layer;
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
    despawn(layer_manager.canvas_layer.camera);
    despawn(layer_manager.canvas_layer.background_entity);
    despawn(layer_manager.blur_layer.layer.camera);
    despawn(layer_manager.blur_layer.layer.background_entity);

    if let Some(mut camera) = world.get_mut::<Camera>(entity) {
        camera.target = layer_manager.render_target;
    }
}

fn rect_area(rect: Rect) -> f32 {
    rect.width() * rect.height()
}

pub fn merge_rects(rects: impl IntoIterator<Item = Rect>) -> Vec<Rect> {
    let mut result = Vec::<Rect>::new();

    for rect in rects {
        let mut merged = false;
        for existing in result.iter_mut() {
            let union = existing.union(rect);
            if rect_area(union) <= rect_area(*existing) + rect_area(rect) + 1.0 {
                *existing = union;
                merged = true;
                break;
            }
        }
        if !merged {
            result.push(rect);
        }
    }

    result
}

pub fn update_layers(
    mut layer_manager_query: Query<(Entity, &mut LayerManager)>,
    mut camera_query: CameraQuery,
    window_query: Query<&Window>,
    ui_area_query: Query<
        (
            &InheritedVisibility,
            &ComputedNode,
            &GlobalTransform,
            &ComputedNodeTarget,
        ),
        With<LayerRenderArea>,
    >,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let primary_window = primary_window.iter().next();
    let mut layer_rects = HashMap::<Entity, Vec<Rect>>::new();
    for (visibility, computed_node, global_transform, node_camera) in &ui_area_query {
        if !**visibility {
            continue;
        }
        let Some(layer_camera) = node_camera.camera() else {
            continue;
        };
        let rects = layer_rects.entry(layer_camera).or_insert(vec![]);
        let mut rect =
            Rect::from_center_size(global_transform.translation().xy(), computed_node.size());

        if (rect.width() <= 0.0) || (rect.height() <= 0.0) {
            continue;
        }

        rects.push(rect);
    }

    for (entity, mut layer_manager) in &mut layer_manager_query {
        let mut image_size_changed = false;
        {
            let (camera, _, _) = camera_query.camera.get(entity).unwrap();

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
                "layer_manager/base_layer/surface",
            );
            let size = layer_manager.size;
            layer_manager.canvas_layer.update_size(
                size,
                &mut images,
                "layer_manager/canvas_layer/background_image",
            );
            layer_manager.blur_layer.update_size(
                size,
                &mut images,
                "layer_manager/blur_layer/background_image",
            );
        }

        {
            let mut render_target = layer_manager.render_target.clone();
            layer_manager.blur_layer.layer.update_camera(
                layer_manager.blur_enable,
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
                layer_manager.size.as_vec2(),
            );
            layer_manager.canvas_layer.update_camera(
                layer_manager.canvas_enable,
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
                layer_manager.size.as_vec2(),
            );
            layer_manager.base_layer.update_camera(
                &mut camera_query,
                &mut render_target,
                &layer_manager.window_target,
            );
        }

        let surface_size = layer_manager.size;
        {
            if layer_manager.blur_enable {
                if let Some(rects) = layer_rects.get(&layer_manager.blur_layer.layer.camera) {
                    layer_manager
                        .blur_layer
                        .update_rects(rects, surface_size, &mut meshes);
                }
            }
            if layer_manager.canvas_enable {
                if let Some(rects) = layer_rects.get(&layer_manager.canvas_layer.camera) {
                    layer_manager
                        .canvas_layer
                        .update_rects(rects, surface_size, &mut meshes);
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
            .add_systems(
                PreUpdate,
                (
                    before_ui_focus_system
                        .in_set(UiSystem::Focus)
                        .before(ui_focus_system),
                    after_ui_focus_system
                        .in_set(UiSystem::Focus)
                        .after(ui_focus_system),
                )
                    .in_set(UiSystem::Focus),
            )
            .add_systems(
                Last,
                (update_layers.in_set(UiFrameworkSystems::UpdateLayers),).chain(),
            );
    }
}
