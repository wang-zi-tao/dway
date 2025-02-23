pub mod gles;
pub mod vulkan;

use anyhow::{anyhow, Error, Result};
use ash::vk;
use bevy::{
    ecs::entity::EntityHashMap,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        renderer::RenderDevice,
        texture::GpuImage,
        Extract, Render, RenderApp, RenderSet,
    },
    utils::{hashbrown::hash_map::Entry, HashMap},
};
use drm::control::framebuffer;
use drm_fourcc::DrmFormat;
use tracing::{span, Level};
use wgpu::core::hal_api::HalApi;
use wgpu_hal::api::Gles;

use self::gles::GlesRenderFunctions;
use crate::{
    drm::{
        connectors::Connector,
        surface::DrmSurface,
        DrmDevice,
    },
    gbm::GbmDevice,
};

#[derive(thiserror::Error, Debug)]
pub enum TtyRenderError {
    #[error("wgpu abi is dismatch")]
    WgpuAbiDisMatch,
    #[error("gpu backend is not egl")]
    BackendIsNotEGL,
    #[error("gpu backend is invalid")]
    BackendIsIsInvalid,
    #[error("gpu backend is not vulkan")]
    BackendIsNotVulkan,
    #[error("the hal texture is invalid")]
    EglInstanceIsNotInitialized,
    #[error("egl instance is not initialized")]
    HalTextureIsInvalid,
    #[error("failed to create swapchain: {0}")]
    CreateSwapchain(Error),
    #[error("failed to acquire surface: {0}")]
    AcquireSurface(Error),
    #[error("failed to discard surface: {0}")]
    DiscardSurface(Error),
    #[error("failed to copy image: {0}")]
    CopyImage(Error),
    #[error("unknown error when calling {0}")]
    EglApiError(&'static str),
    #[error("gl error: {0:?}")]
    GLError(u32),
    #[error("egl error: {0:?}")]
    EglError(#[from] khronos_egl::Error),
    #[error("vulkan error: {0:?}")]
    VKError(#[from] vk::Result),
    #[error("unknown egl error")]
    UnknownEglError,
    #[error("{0}")]
    Unknown(#[from] anyhow::Error),
}

#[derive(Resource, Default)]
pub struct TtyRenderState {
    pub buffers: HashMap<framebuffer::Handle, GpuImage>,
    pub entity_map: EntityHashMap<Entity>,
    pub formats: Option<Vec<DrmFormat>>,
    pub cache: RenderCache,
}

#[derive(Default)]
pub enum RenderCache {
    #[default]
    None,
    Gles(GlesRenderFunctions),
}

pub struct TtyRenderPlugin;
impl Plugin for TtyRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<TtyRenderState>()
                .add_systems(ExtractSchedule, extract_drm_surfaces)
                .add_systems(
                    Render,
                    (init_render, apply_deferred)
                        .run_if(run_once)
                        .in_set(RenderSet::PrepareResources),
                )
                .add_systems(
                    Render,
                    commit_drm_surface::<gles::GlTtyRender>
                        .run_if(resource_exists::<TtySwapchains<gles::GlTtyRender>>)
                        .in_set(RenderSet::Cleanup),
                )
            ;
        }
    }
}

pub trait TtyRender: Send + Sync + 'static {
    type Swapchain: Send + Sync;
    type Surface: Send + Sync;
    type Api: HalApi;

    fn new(device: &<Self::Api as wgpu_hal::Api>::Device) -> Result<Self>
    where
        Self: Sized;

    unsafe fn create_swapchain(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        drm_surface: &DrmSurface,
        drm: &DrmDevice,
        gbm: &GbmDevice,
    ) -> Result<Self::Swapchain>;

    unsafe fn acquire_surface(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        swapchain: &mut Self::Swapchain,
    ) -> Result<Self::Surface>;

    unsafe fn discard_surface(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        surface: Self::Surface,
    ) -> Result<()>;

    unsafe fn copy_image(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        surface: &mut Self::Surface,
        image: &<Self::Api as wgpu_hal::Api>::Texture,
    ) -> Result<()>;

    unsafe fn commit(
        &mut self,
        swapchain: &mut Self::Swapchain,
        surface: &mut Self::Surface,
        drm_surface: &DrmSurface,
        drm: &DrmDevice,
    ) -> Result<()>;
}

#[derive(Resource)]
pub struct TtySwapchains<R: TtyRender> {
    pub swapchains: EntityHashMap<R::Swapchain>,
    pub render: R,
    pub inited: bool,
}

impl<R: TtyRender> TtySwapchains<R> {
}

pub fn init_render(render_device: Res<RenderDevice>, mut commands: Commands) {
    let device = render_device.wgpu_device();
    unsafe {
        let finish = device
            .as_hal::<Gles, _, _>(|hal_device| {
                let Some(hal_device) = hal_device else {
                    return false;
                };
                match gles::GlTtyRender::new(hal_device) {
                    Err(e) => {
                        error!("failed to create render with gles: {e}");
                        false
                    }
                    Ok(o) => {
                        commands.insert_resource(TtySwapchains {
                            swapchains: EntityHashMap::default(),
                            render: o,
                            inited: true,
                        });
                        true
                    }
                }
            })
            .unwrap_or(false);
        if !finish {
            panic!("failed to create tty render");
        }
    };
}

pub fn commit_drm_surface<R: TtyRender>(
    surface_query: Query<(Entity, &DrmSurface, &Parent)>,
    drm_query: Query<(&DrmDevice, &GbmDevice)>,
    render_device: Res<RenderDevice>,
    mut state: ResMut<TtySwapchains<R>>,
    render_images: ResMut<RenderAssets<GpuImage>>,
) {
    //for entity in despawned_surface.read() {
    //    state.swapchains.remove(&entity);
    //}
    let TtySwapchains::<R> {
        swapchains, render, ..
    } = &mut *state;

    for (entity, drm_surface, drm_entity) in surface_query.iter() {
        let Ok((drm, gbm)) = drm_query.get(drm_entity.get()) else {
            error!("drm or gbm device not found");
            continue;
        };
        let _span =
            span!(Level::ERROR,"commit drm buffer",device=%drm.path.to_string_lossy()).entered();

        let swapchain = match swapchains.entry(entity) {
            Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
            Entry::Vacant(vacant_entry) => {
                let result = unsafe {
                    render_device
                        .wgpu_device()
                        .as_hal::<R::Api, _, _>(|hal_device| {
                            let hal_device =
                                hal_device.ok_or_else(|| TtyRenderError::BackendIsNotEGL)?;
                            Ok(render.create_swapchain(hal_device, drm_surface, drm, gbm)?)
                        })
                        .ok_or_else(|| TtyRenderError::WgpuAbiDisMatch)
                        .flatten()
                };

                match result {
                    Ok(o) => vacant_entry.insert(o),
                    Err(e) => {
                        error!("failed to create swapchain: {e}");
                        continue;
                    }
                }
            }
        };

        if let Err(e) = unsafe {
            render_device
                .wgpu_device()
                .as_hal::<R::Api, _, _>(|hal_device| {
                    let hal_device = hal_device.ok_or_else(|| TtyRenderError::BackendIsNotEGL)?;
                    let mut surface = render.acquire_surface(hal_device, swapchain)?;

                    let gpu_image = render_images
                        .get(drm_surface.image.id())
                        .ok_or_else(|| anyhow!("surface image not found"))?;

                    gpu_image.texture.as_hal::<R::Api, _, _>(|hal_texture| {
                        let hal_texture =
                            hal_texture.ok_or_else(|| TtyRenderError::HalTextureIsInvalid)?;
                        render.copy_image(hal_device, &mut surface, hal_texture)
                    })?;
                    debug!("copy image success");

                    render.commit(swapchain, &mut surface, drm_surface, drm)?;

                    Ok(())
                })
                .ok_or_else(|| TtyRenderError::WgpuAbiDisMatch)
                .flatten()
        } {
            error!("failed to commit drm surface: {e}");
            continue;
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
    drm_query
        .iter()
        .for_each(|(entity, drm_device, gbm_device)| {
            let render_entity = commands
                .spawn((drm_device.clone(), gbm_device.clone()))
                .id();
            state.entity_map.insert(entity, render_entity);
        });
    surface_query.iter().for_each(|(surface, conn, parent)| {
        let mut entity_command = commands.spawn((surface.clone(), conn.clone()));
        if let Some(parent_entity) = state.entity_map.get(&parent.get()) {
            entity_command.set_parent(*parent_entity);
        } else {
            error!("parent entity is not extracted");
        }
    });
}
