use bevy::{
    math::FloatOrd,
    prelude::*,
    render::camera::{ImageRenderTarget, RenderTarget},
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

pub fn before_ui_focus(mut query: Query<(&DrmCamera, &mut Camera)>) {
    for (drm_info, mut camera) in &mut query {
        camera.target = RenderTarget::Window(WindowRef::Entity(drm_info.drm_surface));
    }
}

pub fn after_ui_focus(mut query: Query<(&DrmCamera, &mut Camera)>) {
    for (drm_info, mut camera) in &mut query {
        camera.target = RenderTarget::Image(ImageRenderTarget {
            handle: drm_info.image_handle.clone(),
            scale_factor: FloatOrd(1.0),
        });
    }
}
