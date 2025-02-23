use bevy::{
    prelude::*,
    render::{
        camera::{RenderTarget, ScalingMode},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
};
use const_fnv1a_hash::fnv1a_hash_16_xor;
use smart_default::SmartDefault;

#[derive(Component, Debug, Clone, Reflect, SmartDefault)]
pub struct UiCanvas {
    image: Handle<Image>,
    size: Vec2,
    refresh: bool,
    #[default(true)]
    reuse_image: bool,
}

#[derive(Component, Debug)]
pub struct CanvasCamera {
    pub canvas: Entity,
}

impl UiCanvas {
    pub fn new_no_reuse() -> Self {
        Self {
            reuse_image: false,
            ..Default::default()
        }
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn set_refresh(&mut self, refresh: bool) {
        self.refresh = refresh;
    }

    pub fn set_image(&mut self, image: Handle<Image>) {
        self.image = image;
    }

    pub fn reuse_image(&self) -> bool {
        self.reuse_image
    }

    pub fn set_reuse_image(&mut self, reuse_image: bool) {
        self.reuse_image = reuse_image;
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
        let alloc_size = size * 16.0;
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
        DEFAULT_CANVAS_RENDER_AREA_BEGIN.x + 65536,
        DEFAULT_CANVAS_RENDER_AREA_BEGIN.y + 65536,
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
    continue_rending: bool,
}

impl UiCanvasRenderCommand {
    pub fn camera(&self) -> Entity {
        self.camera
    }

    pub fn transform(&self) -> Transform {
        self.transform
    }

    pub fn continue_rending(&mut self) {
        self.continue_rending = true;
    }
}

pub fn prepare_render_command(
    mut canvas_query: Query<
        (Entity, &mut UiCanvas, &mut ImageNode, &ComputedNode),
        Or<(Changed<UiCanvas>, Changed<ImageNode>, Changed<Node>)>,
    >,
    mut images: ResMut<Assets<Image>>,
    mut render_area: ResMut<UiCanvasRenderArea>,
    mut commands: Commands,
) {
    for (entity, mut canvas, mut ui_image, compulted_node) in canvas_query.iter_mut() {
        let _span = info_span!("prepare_canvas", ?entity).entered();
        let node_size = compulted_node.size();
        if canvas.size != node_size || canvas.refresh || canvas.image.is_weak() {
            canvas.refresh = false;
            canvas.size = compulted_node.size();
            if node_size.x > 0.0 && node_size.y > 0.0 {
                let handle = if node_size != canvas.size()
                    || !canvas.reuse_image
                    || canvas.image.is_weak()
                {
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
                    ..OrthographicProjection::default_2d()
                };
                let camera_entity = commands
                    .spawn((
                        Camera2dBundle {
                            camera: Camera {
                                order: -(render_area.alloc_count as isize),
                                target: RenderTarget::Image(handle),
                                ..default()
                            },
                            camera_2d: Camera2d,
                            projection,
                            transform,
                            ..default()
                        },
                        // UiCameraConfig { show_ui: false }, TODO
                        CanvasCamera { canvas: entity },
                    ))
                    .id();
                commands.entity(entity).insert(UiCanvasRenderCommand {
                    camera: camera_entity,
                    transform,
                    continue_rending: false,
                });
            }
        };
        if ui_image.image != canvas.image {
            ui_image.image = canvas.image.clone();
        }
    }
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
    mut render_stub_query: Query<(Entity, &mut UiCanvasRenderCommand, &mut ImageNode)>,
    camera_query: Query<(Entity, &CanvasCamera)>,
    mut commands: Commands,
) {
    for (e, mut rendercommand, mut image) in render_stub_query.iter_mut() {
        image.set_changed();
        if !rendercommand.continue_rending {
            commands.entity(rendercommand.camera).despawn();
            commands.entity(e).remove::<UiCanvasRenderCommand>();
        } else {
            rendercommand.continue_rending = false;
        }
    }
    for (entity, camera) in camera_query.iter() {
        if !render_stub_query.contains(camera.canvas) {
            commands.entity(entity).despawn_recursive();
        }
    }
}
