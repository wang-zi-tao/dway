use bevy::{
    camera::{ImageRenderTarget, RenderTarget},
    prelude::*,
    window::WindowRef,
};

use super::surface::DrmSurface;

#[derive(Component, Reflect, Debug, Clone)]
pub struct DrmCamera {
    pub image_handle: Handle<Image>,
    pub drm_surface: Entity,
}

impl DrmCamera {
    pub fn new(drm_surface_entity: Entity, drm_surface: &DrmSurface) -> Self {
        Self {
            image_handle: drm_surface.image(),
            drm_surface: drm_surface_entity,
        }
    }
}

pub fn before_ui_focus(
    mut query: Query<(Entity, &DrmCamera)>,
    mut commands: Commands,
) {
    for (entity, drm_info) in &mut query {
        commands.entity(entity).insert(RenderTarget::Window(WindowRef::Entity(drm_info.drm_surface)));
    }
}

pub fn after_ui_focus(
    mut query: Query<(Entity, &DrmCamera)>,
    mut commands: Commands,
) {
    for (entity, drm_info) in &mut query {
        commands.entity(entity).insert(RenderTarget::Image(ImageRenderTarget {
            handle: drm_info.image_handle.clone(),
            scale_factor: 1.0,
        }));
    }
}
