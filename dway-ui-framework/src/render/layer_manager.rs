use crate::{
    prelude::*,
    shader::{
        fill::Fill,
        shape::{RoundedRect, Shape},
        BindGroupBuilder, BindGroupLayoutBuilder, BuildBindGroup, ShaderAsset, ShaderBuilder,
        ShaderPlugin, ShaderVariables, ShapeRender, UniformLayout,
    },
    widgets::util::visibility,
};
use bevy::{
    asset::load_internal_asset,
    ecs::{
        entity::EntityHashSet,
        system::{Command, EntityCommand, Resource},
    },
    render::{
        camera::{NormalizedRenderTarget, RenderTarget},
        mesh::{Indices, PrimitiveTopology},
        render_asset::{RenderAssetUsages, RenderAssets},
        render_resource::{
            AsBindGroupError, Extent3d, TextureDescriptor, TextureDimension, TextureFormat,
            TextureUsages,
        },
    },
    sprite::{Mesh2d, Mesh2dHandle},
    transform::components::Transform,
    utils::{petgraph::algo::kosaraju_scc, HashMap},
    window::{PrimaryWindow, WindowRef},
};
use bevy_prototype_lyon::entity::{Path, ShapeBundle};
use bevy_relationship::reexport::{Entity, SmallVec};

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    #[default]
    Normal,
    Blur,
    Canvas,
}

#[derive(Component, Default)]
pub struct RenderToLayer {
    pub layer_manager: Option<Entity>,
    pub layer_camera: Option<Entity>,
    pub background_texture: Handle<Image>,
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

#[derive(Component)]
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

#[derive(Debug)]
pub(crate) struct BaseLayerRef {
    pub(crate) camera: Entity,
    surface: Handle<Image>,
}

pub trait Layer {
    fn new(
        world: &mut World,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self;

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>);

    fn update_render_target(
        &self,
        enable: bool,
        camera_query: &mut Query<&mut Camera>,
        render_target: &mut RenderTarget,
    );

    fn update_background(
        &mut self,
        enable: bool,
        background_query: &mut Query<(&mut Handle<Image>, &mut Visibility)>,
        background: &mut Handle<Image>,
    );

    fn update_rects(&mut self, rects: &[Rect], meshes: &mut Assets<Mesh>);
}

#[derive(Debug)]
pub(crate) struct LayerRef {
    pub(crate) camera: Entity,
    pub(crate) background_entity: Entity,
    pub(crate) background_image: Handle<Image>,
    pub(crate) surface: Handle<Image>,
    pub(crate) area: Handle<Mesh>,
}

impl Layer for LayerRef {
    fn new(
        world: &mut World,
        order: isize,
        render_target: &RenderTarget,
        manager_entity: Entity,
    ) -> Self {
        let mut image_assets = world.resource_mut::<Assets<Image>>();
        let size = UVec2::ONE;
        let surface = create_image(size, &mut image_assets);
        let camera_entity = world
            .spawn((
                Camera2dBundle {
                    camera: Camera {
                        target: render_target.clone(),
                        order,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                LayerCamera {
                    layer_manager: manager_entity,
                    layer_kind: LayerKind::Blur,
                },
            ))
            .set_parent(manager_entity)
            .id();
        let background_entity = world
            .spawn(SpriteBundle {
                transform: Transform::from_xyz(0.0, 0.0, -4095.0),
                ..Default::default()
            })
            .id();
        LayerRef {
            camera: camera_entity,
            background_entity,
            surface,
            area: Default::default(),
            background_image: Default::default(),
        }
    }

    fn update_render_target(
        &self,
        enable: bool,
        camera_query: &mut Query<&mut Camera>,
        render_target: &mut RenderTarget,
    ) {
        let mut camera = camera_query.get_mut(self.camera).unwrap();
        camera.target = render_target.clone();
        camera.is_active = enable;
        if enable {
            *render_target = RenderTarget::Image(self.surface.clone());
        }
    }

    fn update_background(
        &mut self,
        enable: bool,
        background_query: &mut Query<(&mut Handle<Image>, &mut Visibility)>,
        background: &mut Handle<Image>,
    ) {
        if enable {
            let (mut image, mut vis) = background_query.get_mut(self.background_entity).unwrap();
            *vis = visibility(enable);
            *image = background.clone();
            self.background_image = background.clone();
            *background = self.surface.clone();
        }
    }

    fn update_size(&mut self, size: UVec2, images: &mut Assets<Image>) {
        update_image(size, &self.surface, images);
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
    #[derive(Debug)]
    pub(crate) struct BlurLayer{
        pub(crate) blur_method:
            #[derive(Clone, Copy, Reflect, Debug)]
            pub enum BlurMethod {
                Kawase{ layer: usize, radius: f32 },
                Dual{ layer: usize, radius: f32 },
                // Gaussian{ kernel_size: usize},
            },
        pub(crate) layer: LayerRef,
        pub(crate) shader: Handle<Shader>,
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
}

#[derive(Component, Debug)]
pub struct LayerManager {
    pub(crate) base_layer: BaseLayerRef,
    pub(crate) canvas_layer: LayerRef,
    pub(crate) blur_layer: BlurLayer,
    pub(crate) size: UVec2,
    pub(crate) canvas_enable: bool,
    pub(crate) blur_enable: bool,
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
            format: TextureFormat::Bgra8UnormSrgb,
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

fn update_image(size: UVec2, handle: &Handle<Image>, images: &mut Assets<Image>) {
    let image = create_image_descripteor(size);
    images.insert(handle.clone(), image);
}

fn create_image(size: UVec2, images: &mut Assets<Image>) -> Handle<Image> {
    let image = create_image_descripteor(size);
    images.add(image)
}

impl LayerManager {
    pub fn get_background_image(&self, kind: LayerKind) -> Handle<Image> {
        match kind {
            LayerKind::Normal => Default::default(),
            LayerKind::Blur => self.blur_layer.layer.background_image.clone(),
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

    pub fn create(entity: Entity, world: &mut World) {
        let camera = world.get::<Camera>(entity).unwrap();
        let render_target = camera.target.clone();

        let canvas_layer = LayerRef::new(world, 1, &render_target, entity);
        let blur_layer = LayerRef::new(world, 2, &render_target, entity);
        let blur_layer = BlurLayer {
            blur_method: BlurMethod::Dual {
                layer: 4,
                radius: 0.5,
            },
            layer: blur_layer,
            shader: Default::default(),
        };

        world.entity_mut(entity).insert(LayerManager {
            base_layer: BaseLayerRef {
                surface: Default::default(),
                camera: entity,
            },
            canvas_layer,
            blur_layer,
            size: UVec2::ONE,
            canvas_enable: false,
            blur_enable: false,
            render_target,
        });
    }
}

pub fn update_ui_root(
    mut query: Query<(Entity, &mut RenderToLayer, &mut TargetCamera, &mut Style, &Node, &GlobalTransform), With<Parent>>,
    mut commmands: Commands,
    layer_manager_query: Query<&LayerManager>,
) {
    for (entity, mut render_to_layer, mut target_camera, mut style, node, global_transform) in &mut query {
        let layer_manager = target_camera.0;
        render_to_layer.layer_manager = Some(layer_manager);
        let Ok(layer_manager) = layer_manager_query.get(layer_manager) else {
            commmands.entity(layer_manager).add(LayerManager::create);
            continue;
        };

        let layer_camera = layer_manager.get_camera(render_to_layer.layer_kind);
        render_to_layer.layer_camera = Some(layer_camera);
        target_camera.0 = layer_camera;
        commmands.entity(entity).remove_parent();

        render_to_layer.background_size = layer_manager.size.as_vec2();
        render_to_layer.background_texture =
            layer_manager.get_background_image(render_to_layer.layer_kind);

        style.position_type = PositionType::Absolute;
        let rect = node.logical_rect(global_transform);
        style.left = Val::Px(rect.min.x);
        style.top = Val::Px(rect.min.y);
        style.width = Val::Px(rect.width());
        style.height = Val::Px(rect.height());
        style.right = Val::Auto;
        style.bottom = Val::Auto;
    }
}

pub fn update_layers(
    mut layer_manager_query: Query<(Entity, &mut LayerManager)>,
    mut camera_query: Query<&mut Camera>,
    mut background_query: Query<(&mut Handle<Image>, &mut Visibility)>,
    window_query: Query<&Window>,
    ui_root_query: Query<
        (&ViewVisibility, &RenderToLayer, &Node, &GlobalTransform),
        With<RenderToLayer>,
    >,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let primary_window = primary_window.iter().next();
    let mut layer_rects = HashMap::<Entity, Vec<Rect>>::new();
    for (visibility, render_to_layer, node, global_transform) in &ui_root_query {
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
        let mut rect = node.logical_rect(global_transform);
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
            let camera = camera_query.get(entity).unwrap();

            let size = match camera.target.normalize(primary_window) {
                Some(NormalizedRenderTarget::Window(window_ref)) => window_query
                    .get(window_ref.entity())
                    .map(|w| UVec2::new(w.physical_width(), w.physical_height()))
                    .unwrap_or(UVec2::ONE),
                Some(NormalizedRenderTarget::Image(image)) => {
                    images.get(image).map(Image::size).unwrap_or(UVec2::ONE)
                }
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
                &layer_manager.base_layer.surface,
                &mut images,
            );
            update_image(
                layer_manager.size,
                &layer_manager.canvas_layer.surface,
                &mut images,
            );
            update_image(
                layer_manager.size,
                &layer_manager.blur_layer.layer.surface,
                &mut images,
            );
        }

        {
            let mut render_target = layer_manager.render_target.clone();
            layer_manager.blur_layer.layer.update_render_target(
                layer_manager.blur_enable,
                &mut camera_query,
                &mut render_target,
            );
            layer_manager.canvas_layer.update_render_target(
                layer_manager.canvas_enable,
                &mut camera_query,
                &mut render_target,
            );
            let mut camera = camera_query.get_mut(entity).unwrap();
            camera.target = render_target;
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
            layer_manager.blur_layer.layer.update_background(
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
    fn to_wgsl(builder: &mut ShaderBuilder, var: &ShaderVariables) -> String {
        let ShaderVariables { pos, size } = var;
        let var_image_texture = builder.get_binding("background_texture", "", "texture_2d<f32>");
        let var_image_sampler = builder.get_binding("background_sampler", "", "sampler");
        let uniform_size = builder.get_uniform("background_size", "", "vec2<f32>");
        format!("textureSample({var_image_texture}, {var_image_sampler}, {pos}/{uniform_size})")
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

    fn write_uniform<B: encase::internal::BufferMut>(
        &self,
        layout: &mut UniformLayout,
        writer: &mut encase::internal::Writer<B>,
    ) {
        layout.write_uniform(&self.texture_size, writer);
    }
}

type FillWithLayerMaterial = ShapeRender<RoundedRect, FillWithLayer>;

pub fn update_ui_material(
    mut query: Query<(
        &RenderToLayer,
        &mut Handle<ShaderAsset<FillWithLayerMaterial>>,
    )>,
    mut material_assets: ResMut<Assets<ShaderAsset<FillWithLayerMaterial>>>,
) {
    for (render_to_layer, mut shader_handle) in &mut query {
        let material = RoundedRect::new(16.0).with_effect(FillWithLayer {
            texture: render_to_layer.background_texture.clone(),
            texture_size: render_to_layer.background_size,
        });
        *shader_handle = material_assets.add(material);
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
        app.add_plugins(ShaderPlugin::<FillWithLayerMaterial>::default())
            .add_systems(
                Last,
                (update_ui_root, update_layers, update_ui_material)
                    .chain()
                    .in_set(UiFrameworkSystems::UpdateLayers),
            );
    }
}
