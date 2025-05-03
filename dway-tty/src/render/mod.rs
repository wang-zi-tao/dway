pub mod gles;
pub mod vulkan;

use anyhow::{anyhow, Error, Result};
use ash::vk;
use bevy::{
    ecs::entity::EntityHashMap,
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        renderer::RenderDevice,
        sync_component::SyncComponentPlugin,
        sync_world::{MainEntity, RenderEntity, SyncToRenderWorld, TemporaryRenderEntity},
        texture::GpuImage,
        Extract, Render, RenderApp, RenderSet,
    },
    ui::ExtractedUiItem,
    utils::{hashbrown::hash_map::Entry, HashMap},
};
use drm::control::framebuffer;
use drm_fourcc::DrmFormat;
use dway_util::temporary::TemporaryEntity;
use tracing::{span, Level};
use wgpu::core::hal_api::HalApi;
use wgpu_hal::api::Gles;

use self::gles::GlesRenderFunctions;
use crate::{
    drm::{connectors::Connector, surface::DrmSurface, DrmDevice, ExtractedDrmDevice},
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
    TextureIsNotValid,
    #[error("texture is not valid")]
    FrameBufferIsNotValid,
    #[error("frame buffer is not valid")]
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
        app.add_plugins(SyncComponentPlugin::<DrmSurface>::default());
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
                );
        }
    }
}

pub trait TtyRender: Send + Sync + 'static {
    type Swapchain: Send + Sync;
    type Surface: Send + Sync;
    type Api: HalApi;

    fn new(device: &<Self::Api as wgpu_hal::Api>::Device) -> Result<Self, TtyRenderError>
    where
        Self: Sized;

    unsafe fn create_swapchain(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        drm_surface: &DrmSurface,
        drm: &DrmDevice,
        gbm: &GbmDevice,
    ) -> Result<Self::Swapchain, TtyRenderError>;

    unsafe fn acquire_surface(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        swapchain: &mut Self::Swapchain,
    ) -> Result<Self::Surface, TtyRenderError>;

    unsafe fn discard_surface(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        surface: Self::Surface,
    ) -> Result<(), TtyRenderError>;

    unsafe fn copy_image(
        &mut self,
        device: &<Self::Api as wgpu_hal::Api>::Device,
        surface: &mut Self::Surface,
        image: &<Self::Api as wgpu_hal::Api>::Texture,
    ) -> Result<(), TtyRenderError>;

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
    surface_query: Query<(Entity, &ExtractedDrmSurface)>,
    drm_query: Query<&ExtractedDrmDevice>,
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

    for (entity, extracted_drm_surface) in surface_query.iter() {
        let ExtractedDrmSurface {
            surface: drm_surface,
            device_entity: drm_entity,
            ..
        } = extracted_drm_surface;
        let Ok(ExtractedDrmDevice { device: drm, gbm }) = drm_query.get(*drm_entity) else {
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

                    render.commit(swapchain, &mut surface, drm_surface, drm)?;

                    render.discard_surface(hal_device, surface)?;

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

#[derive(Component)]
pub struct ExtractedDrmSurface {
    pub surface: DrmSurface,
    pub connector: Connector,
    pub device_entity: Entity,
}

#[tracing::instrument(skip_all)]
pub fn extract_drm_surfaces(
    surface_query: Extract<
        Query<(&DrmSurface, &Connector, &Parent, RenderEntity)>,
    >,
    drm_query: Extract<Query<RenderEntity>>,
    mut commands: Commands,
) {
    surface_query
        .iter()
        .for_each(|(surface, conn, parent, render_entity)| {
            let Ok(drm_device_entity) = drm_query.get(parent.get()) else {
                todo!();//TODO
                return;
            };
            commands.entity(render_entity).insert(ExtractedDrmSurface {
                surface: surface.clone(),
                connector: conn.clone(),
                device_entity: drm_device_entity,
            });
        });
}
