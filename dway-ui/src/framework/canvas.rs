use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    math::IRect,
    prelude::*,
    render::{
        camera::{CameraProjection, RenderTarget, ScalingMode},
        primitives::Frustum,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        texture::{BevyDefault, ImageSampler},
        view::RenderLayers,
    },
};
use bevy_vector_shapes::{
    painter::CanvasBundle,
    prelude::{Canvas, CanvasConfig, CanvasMode, ShapePainter},
    render::ShapePipelineType,
    shapes::RectPainter,
};
use const_fnv1a_hash::fnv1a_hash_16_xor;

#[derive(Component, Debug, Clone, Reflect, Default)]
pub struct UiCanvas {
    image: Handle<Image>,
    size: Vec2,
    refresh: bool,
}

impl UiCanvas {
    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_refresh(&mut self, refresh: bool) {
        self.refresh = refresh;
    }

    pub fn setup_painter(
        &self,
        render_command: &UiCanvasRenderCommand,
        painter: &mut ShapePainter,
    ) {
        painter.reset();
        painter.image(self.image.clone(), Vec2::splat(1.0));
        painter.set_translation(render_command.transform().translation);
        painter.pipeline = ShapePipelineType::Shape2d;
    }
}

#[derive(Resource)]
pub struct UiCanvasRenderArea {
    pub rect: Rect,
    pub line_to_alloc: Rect,
    pub alloc_count: usize,
}
impl UiCanvasRenderArea {
    pub fn alloc(&mut self, size: Vec2) -> Rect {
        let alloc_size = size * 1.5;
        self.alloc_count += 1;
        if self.line_to_alloc.max.x + alloc_size.x < self.rect.max.x {
            self.line_to_alloc.max = Vec2::new(
                self.line_to_alloc.max.x + alloc_size.x,
                self.line_to_alloc.max.y.max(self.rect.min.y + alloc_size.y),
            );
            Rect::new(
                self.line_to_alloc.max.x,
                self.line_to_alloc.min.y,
                self.line_to_alloc.max.x + size.x,
                self.line_to_alloc.min.y + size.y,
            )
        } else {
            self.line_to_alloc.min = Vec2::new(self.rect.min.x, self.line_to_alloc.max.y);
            self.line_to_alloc.max = self.line_to_alloc.min + alloc_size;
            Rect::from_corners(self.line_to_alloc.min, self.line_to_alloc.min + size)
        }
    }

    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            line_to_alloc: Rect::from_corners(rect.min, rect.min),
            alloc_count: 0,
        }
    }

    pub fn clear(&mut self) {
        self.line_to_alloc.min = self.rect.min;
        self.line_to_alloc.max = self.rect.min;
        self.alloc_count = 0;
    }
}
const DEFAULT_CANVAS_RENDER_AREA_BEGIN: IVec2 = IVec2::new(
    65536 + fnv1a_hash_16_xor(b"dway_ui::framework::canvas::x", None) as i32,
    65536 + fnv1a_hash_16_xor(b"dway_ui::framework::canvas::y", None) as i32,
);
const DEFAULT_CANVAS_RENDER_AREA: IRect = IRect {
    min: DEFAULT_CANVAS_RENDER_AREA_BEGIN,
    max: IVec2::new(
        DEFAULT_CANVAS_RENDER_AREA_BEGIN.x + 4096,
        DEFAULT_CANVAS_RENDER_AREA_BEGIN.y + 4096,
    ),
};
impl Default for UiCanvasRenderArea {
    fn default() -> Self {
        Self::new(DEFAULT_CANVAS_RENDER_AREA.as_rect())
    }
}

#[derive(Component, Debug, Clone, Reflect)]
#[component(storage = "SparseSet")]
pub struct UiCanvasRenderCommand {
    camera: Entity,
    transform: Transform,
}

impl UiCanvasRenderCommand {
    pub fn camera(&self) -> Entity {
        self.camera
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }
}

pub fn prepare_render_command(
    mut canvas_query: Query<
        (Entity, &mut UiCanvas, &mut UiImage, &Node),
        Or<(Changed<UiCanvas>, Changed<UiImage>, Changed<Node>)>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut render_area: ResMut<UiCanvasRenderArea>,
    mut commands: Commands,
) {
    canvas_query.for_each_mut(|(entity, mut canvas, mut ui_image, node)| {
        let _span = info_span!("prepare_canvas", ?entity).entered();
        let node_size = node.size();
        if canvas.size != node_size || canvas.refresh || canvas.image.is_weak() {
            canvas.refresh = false;
            canvas.size = node.size();
            if node_size.x > 0.0 && node_size.y > 0.0 {
                let handle = if node_size != canvas.size() || canvas.image.is_weak() {
                    let size = Extent3d {
                        width: node_size.x as u32 * 2,
                        height: node_size.y as u32 * 2,
                        ..default()
                    };

                    let mut image = Image {
                        texture_descriptor: TextureDescriptor {
                            label: Some("ui-canvas"),
                            size,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::Bgra8UnormSrgb,
                            mip_level_count: 1,
                            sample_count: 1,
                            usage: TextureUsages::TEXTURE_BINDING
                                | TextureUsages::COPY_DST
                                | TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                        sampler_descriptor: ImageSampler::Default,
                        ..default()
                    };
                    image.resize(size);
                    let handle = images.add(image);
                    canvas.image = handle.clone();
                    debug!("create image for canvas: {:?} {:?}", node_size, handle);
                    handle
                } else {
                    debug!("reuse canvas image: {:?} {:?}", node_size, &canvas.image);
                    canvas.image.clone()
                };
                canvas.size = node_size;

                let render_rect = render_area.alloc(node_size);
                let transform = Transform::default().with_translation(Vec3::new(
                    render_rect.center().x,
                    render_rect.center().y,
                    0.0,
                ));

                let projection = OrthographicProjection {
                    far: 32.,
                    near: -32.,
                    scaling_mode: ScalingMode::Fixed {
                        width: node_size.x,
                        height: node_size.y,
                    },
                    ..Default::default()
                };
                let view_projection =
                    projection.get_projection_matrix() * transform.compute_matrix().inverse();
                let frustum = Frustum::from_view_projection_custom_far(
                    &view_projection,
                    &transform.translation,
                    &transform.back(),
                    projection.far(),
                );
                let camera_entity = commands
                    .spawn((
                        Camera2dBundle {
                            camera: Camera {
                                order: -(render_area.alloc_count as isize),
                                target: RenderTarget::Image(handle),
                                ..default()
                            },
                            camera_2d: Camera2d {
                                clear_color: ClearColorConfig::Custom(Color::BLACK.with_a(0.0)),
                            },
                            projection,
                            frustum,
                            transform,
                            ..default()
                        },
                        UiCameraConfig { show_ui: false },
                    ))
                    .id();
                commands.entity(entity).insert(UiCanvasRenderCommand {
                    camera: camera_entity,
                    transform,
                });
            }
        };
        if &ui_image.texture != &canvas.image {
            ui_image.texture = canvas.image.clone();
        }
    });
}

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum UiCanvasSystems {
    Prepare,
}

#[derive(Bundle, Default)]
pub struct UiCanvasBundle {
    image: ImageBundle,
    canvas: UiCanvas,
}

pub fn cleanup_render_command(
    mut render_stub_query: Query<(Entity, &UiCanvasRenderCommand, &mut UiImage)>,
    mut render_area: ResMut<UiCanvasRenderArea>,
    mut commands: Commands,
) {
    render_stub_query.for_each_mut(|(e, rendercommand, mut image)| {
        image.set_changed();
        commands.entity(rendercommand.camera).despawn();
        commands.entity(e).remove::<UiCanvasRenderCommand>();
    });
    render_area.clear();
}
