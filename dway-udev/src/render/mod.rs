pub mod gles;
pub mod utils;
pub mod vulkan;

use anyhow::{anyhow, Result};
use bevy::{
    prelude::*,
    render::{
        render_asset::{PrepareAssetSet, RenderAssets},
        renderer::{RenderDevice, RenderQueue},
        texture::{DefaultImageSampler, GpuImage},
        Extract, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use drm::control::framebuffer;
use drm_fourcc::DrmFormat;
use tracing::span;
use tracing::Level;
use wgpu::Texture;
use wgpu::{TextureFormat, TextureViewDescriptor};
use wgpu_hal::api::Gles;
use wgpu_hal::api::Vulkan;

use crate::drm::connectors::Connector;
use crate::gbm::buffer::GbmBuffer;
use crate::{
    drm::{
        surface::{drm_framebuffer_descriptor, DrmSurface},
        DrmDevice,
    },
    gbm::GbmDevice,
};

use self::gles::GlesRenderCache;

#[derive(Resource, Default)]
pub struct TtyRenderState {
    pub buffers: HashMap<framebuffer::Handle, GpuImage>,
    pub entity_map: HashMap<Entity, Entity>,
    pub formats: Option<Vec<DrmFormat>>,
    pub cache: RenderCache,
}

#[derive(Default)]
pub enum RenderCache {
    #[default]
    None,
    Gles(GlesRenderCache),
}

impl TtyRenderState {
    pub fn get_formats(&mut self, render_device: &wgpu::Device) -> Result<&[DrmFormat]> {
        if self.formats.is_some() {
            return Ok(&**self.formats.as_ref().unwrap());
        }
        let formats = gles::get_formats(&mut self.cache, render_device)
            .or_else(|| vulkan::get_formats(render_device))
            .ok_or_else(|| anyhow!("unknown wgpu backend"))??;
        self.formats = Some(formats);
        Ok(&self.formats.as_ref().unwrap())
    }
}

pub struct TtyRenderPlugin;
impl Plugin for TtyRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TtyRenderState>()
                .add_system(extract_drm_surfaces.in_schedule(ExtractSchedule))
                .add_system(prepare_drm_surface.in_set(PrepareAssetSet::PostAssetPrepare))
                .add_system(commit.after(RenderSet::Render).before(RenderSet::Cleanup));
        }
    }
}

#[tracing::instrument(skip_all)]
pub fn extract_drm_surfaces(
    surface_query: Extract<Query<(&DrmSurface, &Connector, &Parent)>>,
    drm_query: Extract<Query<(Entity, &DrmDevice, &GbmDevice)>>,
    mut state: ResMut<TtyRenderState>,
    mut commands: Commands,
) {
    drm_query.for_each(|(entity, drm_device, gbm_device)| {
        let render_entity = commands
            .spawn((drm_device.clone(), gbm_device.clone()))
            .id();
        state.entity_map.insert(entity, render_entity);
    });
    surface_query.for_each(|(surface, conn, parent)| {
        let mut entity_command = commands.spawn((surface.clone(), conn.clone()));
        if let Some(parent_entity) = state.entity_map.get(&parent.get()) {
            entity_command.set_parent(*parent_entity);
        }
    });
}

#[tracing::instrument(skip_all)]
pub fn prepare_drm_surface(
    mut state: ResMut<TtyRenderState>,
    surface_query: Query<(&DrmSurface, &Parent)>,
    drm_query: Query<(&DrmDevice, &GbmDevice)>,
    mut render_images: ResMut<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    default_sampler: Res<DefaultImageSampler>,
) {
    surface_query.for_each(|(surface, parent)| {
        let Ok((drm, gbm)) = drm_query.get(parent.get()) else {
            return;
        };
        let _span =
            span!(Level::ERROR,"prepare drm buffer",device=%drm.path.to_string_lossy()).entered();

        let Ok(formats) = state
            .get_formats(render_device.wgpu_device())
            .map_err(|e| error!("failed to get gl formats: {e}"))
        else {
            return;
        };

        let mut surface_guard = surface.inner.lock().unwrap();
        let Ok(buffer) = surface_guard
            .get_buffer(drm, gbm, formats)
            .map_err(|e| error!("failed to create gbm buffer: {}", e))
        else {
            return;
        };

        let gpu_image = if let Some(gpu_image) = state.buffers.get(&buffer.framebuffer) {
            if let Some(Err(e))=vulkan::reset_framebuffer(render_device.wgpu_device(), buffer){
                error!("failed to reset framebuffer: {e}");
            }
            gpu_image.clone()
        } else {
            let Ok(texture) =
                create_framebuffer_texture(&mut state, &render_device.wgpu_device(), buffer)
                    .map_err(|e| error!("failed to bind gbm buffer: {e} \n{}", e.backtrace()))
            else {
                return;
            };
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            let gpu_image = GpuImage {
                texture: texture.into(),
                texture_view: texture_view.into(),
                texture_format: TextureFormat::Bgra8UnormSrgb,
                sampler: (**default_sampler).clone(),
                size: buffer.size.as_vec2(),
                mip_level_count: 1,
            };
            info!("create drm image");
            state.buffers.insert(buffer.framebuffer, gpu_image.clone());
            gpu_image
        };
        render_images.insert(surface.image.clone(), gpu_image);
    });
}

#[tracing::instrument(skip_all)]
pub fn create_framebuffer_texture(
    state: &mut TtyRenderState,
    render_device: &wgpu::Device,
    buffer: &mut GbmBuffer,
) -> Result<Texture> {
    unsafe {
        render_device
            .as_hal::<Gles, _, _>(|hal_device| {
                hal_device
                    .map(|hal_device| gles::create_framebuffer_texture(state, hal_device, buffer))
            })
            .map(|r| {
                r.map(|hal_texture| {
                    render_device.create_texture_from_hal::<Gles>(
                        hal_texture,
                        &drm_framebuffer_descriptor(buffer.size),
                    )
                })
            })
            .or_else(|| {
                render_device
                    .as_hal::<Vulkan, _, _>(|hal_device| {
                        hal_device.map(|hal_device| {
                            vulkan::create_framebuffer_texture(hal_device, buffer)
                        })
                    })
                    .map(|r| {
                        r.map(|hal_texture| {
                            render_device.create_texture_from_hal::<Vulkan>(
                                hal_texture,
                                &drm_framebuffer_descriptor(buffer.size),
                            )
                        })
                    })
            })
            .ok_or_else(|| anyhow!("unknown wgpu backend"))?
    }
}

#[tracing::instrument(skip_all)]
pub fn commit(
    surface_query: Query<(&DrmSurface, &Connector, &Parent)>,
    drm_query: Query<&DrmDevice>,
    render_device: Res<RenderDevice>,
) {
    surface_query.for_each(|(surface, conn, parent)| {
        let Ok(drm) = drm_query.get(parent.get()) else {
            return;
        };
        let _span =
            span!(Level::ERROR,"commit drm buffer",device=%drm.path.to_string_lossy()).entered();
        if let Err(e) = gles::commit_drm(surface, render_device.wgpu_device(), conn, drm)
            .or_else(|| vulkan::commit_drm(surface, render_device.wgpu_device(), conn, drm))
            .ok_or_else(|| anyhow!("unknown wgpu backend"))
            .flatten()
        {
            error!("failed to commit surface to drm: {e}");
        };
        surface.finish_frame();
    });
}
