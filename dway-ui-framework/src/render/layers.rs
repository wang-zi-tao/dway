use crate::prelude::*;
use bevy::{
    ecs::system::Resource,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    window::WindowRef,
};
use bevy_relationship::reexport::Entity;

pub enum Operate {
    CreateLayerManager {
        size: Vec2,
        render_target: RenderTarget,
    },
    Enable(LayerKind),
    Disable(LayerKind),
}

#[derive(Event)]
pub struct LayerManagerRequest {
    operate: Operate,
    camera: Entity,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    #[default]
    Normal,
    Blur,
    Canvas,
}

pub struct LayerRef {
    pub camera: Entity,
    pub ui_root: Option<Entity>,
}

pub enum BlurMethod {
    Kawase { pass: usize, radius: f32 },
    DualKawase { pass: usize, radius: f32 },
}

pub struct BlurLayer {
    pub enable: bool,
    pub layer: LayerRef,
}

#[derive(Component)]
pub struct LayerElement {
    pub entity: Entity,
    pub layer_kind: LayerKind,
}

#[derive(Component)]
pub struct LayerManager {
    pub base_layer: LayerRef,
    pub canvas_layer: LayerRef,
    pub blur_layer: BlurLayer,
    pub render_target: RenderTarget,
    pub size: Vec2,
}

impl LayerManager {
    pub fn get_camera(&self, kind: LayerKind) -> Entity {
        match kind {
            LayerKind::Normal => self.base_layer.camera,
            LayerKind::Blur => self.blur_layer.layer.camera,
            LayerKind::Canvas => self.canvas_layer.camera,
        }
    }

    pub fn get_ui_element_bundle(
        &self,
        self_entity: Entity,
        kind: LayerKind,
    ) -> (TargetCamera, LayerElement) {
        (
            TargetCamera(self.get_camera(kind)),
            LayerElement {
                entity: self_entity,
                layer_kind: kind,
            },
        )
    }
}

pub fn init_layers_manager(camera: Entity, commands: &mut Commands) {}

pub fn update_layers(
    mut events: EventReader<LayerManagerRequest>,
    layer_manager_query: Query<&LayerManager>,
    window_query: Query<&Window>,
    removed: RemovedComponents<LayerElement>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    for LayerManagerRequest {
        operate,
        camera: camera_entity,
    } in events.read()
    {
        if let Ok(layer_manager) = layer_manager_query.get(*camera_entity) {
            match operate {
                Operate::Enable(layer) => todo!(),
                Operate::Disable(layer) => todo!(),
                Operate::CreateLayerManager {
                    size,
                    render_target,
                } => {
                    let mut create_image = |size: Vec2| {
                        let image = Image {
                            texture_descriptor: TextureDescriptor {
                                label: None,
                                size: Extent3d {
                                    width: size.x as u32,
                                    height: size.y as u32,
                                    depth_or_array_layers: 1,
                                    ..default()
                                },
                                dimension: TextureDimension::D2,
                                format: TextureFormat::Bgra8UnormSrgb,
                                usage: TextureUsages::RENDER_ATTACHMENT,
                                mip_level_count: 1,
                                sample_count: 1,
                                view_formats: &[],
                            },
                            ..Default::default()
                        };
                        images.add(image)
                    };
                    commands.entity(*camera_entity).with_children(|c| {
                        let base_camera_surface = create_image(*size);
                        let base_camera = c
                            .spawn(Camera2dBundle {
                                camera: Camera {
                                    order: -8,
                                    is_active: true,
                                    target: RenderTarget::Image(base_camera_surface.clone()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .id();

                        let blur_layer = c
                            .spawn(Camera2dBundle {
                                camera: Camera {
                                    order: -4,
                                    is_active: true,
                                    target: render_target.clone(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .id();
                    });
                }
            }
        }
    }
}
