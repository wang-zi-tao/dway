use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::Duration,
};

use bevy::{
    ecs::entity::EntityHashMap,
    render::{
        render_asset::RenderAssets,
        render_graph::Node,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract,
    },
    utils::HashSet,
};
use drm_fourcc::DrmFourcc;
use dway_util::formats::ImageFormat;
use wgpu::{
    core::hal_api, CommandEncoder, CommandEncoderDescriptor, Extent3d, FilterMode,
    ImageCopyTexture, TextureAspect, TextureDimension,
};
use wgpu_hal::{
    api::{Gles, Vulkan},
    MemoryFlags, TextureUses,
};

use super::{
    gles::{self, DestroyBuffer, EglState},
    util::DWayRenderError,
    vulkan::{self, VulkanState},
    DWayServerRenderServer, ImportDmaBufferRequest,
};
use crate::{
    prelude::*,
    render::DWayRenderResponse,
    state::{WaylandDisplayCreated, WaylandDisplayDestroyed},
    util::rect::IRect,
    wl::{
        buffer::{UninitedWlBuffer, WaylandBuffer, WlShmBuffer},
        surface::WlSurface,
    },
    zwp::dmabufparam::DmaBuffer,
};

pub mod graph {
    use bevy::render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub enum Labels2d {
        Import,
    }
}

#[derive(Default, Debug)]
pub enum RenderImage {
    #[default]
    None,
    Gl(),
    Vulkan(crate::render::vulkan::ImageDropGuard),
}

pub enum ImportedBuffer {
    GL(gles::ImageGuard),
    VULKAN(),
}

#[derive(Resource, Default)]
pub struct ImportState {
    pub inner: Option<ImportStateKind>,
    pub image_set: HashSet<Handle<Image>>,
    pub finished: AtomicBool,
    pub callbacks: Vec<wl_callback::WlCallback>,
    pub elapsed: Duration,
    pub imported_image: EntityHashMap<(ImportedBuffer, GpuImage)>,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct ImoprtedBuffers(EntityHashMap<GpuImage>);

#[derive(Debug)]
pub enum ImportStateKind {
    Egl(EglState),
    Vulkan(VulkanState),
}
impl ImportStateKind {
    pub fn new(device: &wgpu::Device) -> Result<Self, DWayRenderError> {
        unsafe {
            if let Some(o) = device
                .as_hal::<wgpu_hal::api::Vulkan, _, _>(|hal_device| {
                    hal_device.map(|_| Self::Vulkan(VulkanState::default()))
                })
                .flatten()
            {
                return Ok(o);
            };
            if let Some(o) = device
                .as_hal::<wgpu_hal::api::Gles, _, _>(|hal_device| {
                    hal_device.map(|hal_device| {
                        let egl_context = hal_device.context();
                        let gl: &glow::Context = &egl_context.lock();
                        let egl: &khronos_egl::DynamicInstance<khronos_egl::EGL1_4> =
                            egl_context.egl_instance().unwrap();
                        Ok(Self::Egl(EglState::new(gl, egl)?))
                    })
                })
                .flatten()
            {
                return o;
            };
            Err(DWayRenderError::UnknownBackend)
        }
    }
}

#[derive(Resource, Default)]
pub struct DWayDisplayHandles {
    pub map: EntityHashMap<DisplayHandle>,
}

#[allow(clippy::too_many_arguments)]
pub fn extract_surface(
    surface_query: Extract<Query<&WlSurface>>,
    buffer_query: Extract<
        Query<(
            Option<&WlShmBuffer>,
            Option<&DmaBuffer>,
            Option<&UninitedWlBuffer>,
        )>,
    >,
    time: Extract<Res<Time>>,
    mut removed_buffer: Extract<RemovedComponents<WaylandBuffer>>,
    mut create_events: Extract<EventReader<WaylandDisplayCreated>>,
    mut destroy_events: Extract<EventReader<WaylandDisplayDestroyed>>,
    mut wayland_map: ResMut<DWayDisplayHandles>,
    mut state: ResMut<ImportState>,
    mut importd_buffer: ResMut<ImoprtedBuffers>,
    mut commands: Commands,
) {
    state.elapsed = time.elapsed();
    state.callbacks.clear();
    for surface in surface_query.iter() {
        if !surface.just_commit {
            continue;
        }
        state
            .callbacks
            .extend(surface.commited.callbacks.iter().cloned());
        let Some(buffer_entity) = surface.commited.buffer else {
            debug!("surface {:?} has no attachment", surface.raw.id());
            continue;
        };
        let _span = debug_span!("extract surface",buffer=?buffer_entity).entered();

        if let Ok((shm_buffer, dma_buffer, egl_buffer)) = buffer_query.get(buffer_entity) {
            if let Some(buffer) = shm_buffer {
                debug!("use shared memory buffer");
                commands.spawn((surface.clone(), buffer.clone()));
            } else if let Some(dma_buffer) = dma_buffer {
                debug!("use dma buffer");
                commands.spawn((surface.clone(), dma_buffer.clone()));
            } else if let Some(egl_buffer) = egl_buffer {
                debug!("use egl buffer");
                commands.spawn((surface.clone(), egl_buffer.clone()));
            } else {
                error!(entity=?buffer_entity,"buffer not found");
            };
        } else {
            error!(entity=?buffer_entity,"buffer not found");
        }

        debug!("extract wayland buffer: {buffer_entity:?}");
        state.image_set.insert(surface.image.clone_weak());
    }
    for WaylandDisplayCreated(entity, display_handle) in create_events.read() {
        wayland_map.map.insert(*entity, display_handle.clone());
    }
    for WaylandDisplayDestroyed(entity, _display_handle) in destroy_events.read() {
        wayland_map.map.remove(entity);
    }
    for entity in removed_buffer.read() {
        importd_buffer.remove(&entity);
    }
}

#[tracing::instrument(skip_all)]
pub fn prepare_surfaces(
    render_device: Res<RenderDevice>,
    mut import_state: ResMut<ImportState>,
    mut imported_images: ResMut<ImoprtedBuffers>,
    mut render_server: ResMut<DWayServerRenderServer>,
) {
    import_state.finished.store(false, Ordering::Relaxed);
    if import_state.inner.is_none() {
        match ImportStateKind::new(render_device.wgpu_device()) {
            Ok(o) => import_state.inner = Some(o),
            Err(e) => {
                error!("failed to prepare wayland surface: {e}");
                return;
            }
        }
    }

    for mut request in std::mem::take(&mut render_server.import_dma_buffer_requests) {
        let buffer = match import_state.inner.as_ref().unwrap() {
            ImportStateKind::Egl(egl_state) => {
                gles::create_wgpu_dma_image(render_device.wgpu_device(), &mut request, egl_state)
            }
            ImportStateKind::Vulkan(_) => {
                vulkan::create_wgpu_dma_image(render_device.wgpu_device(), &mut request)
            }
        }
        .and_then(|gpu_image| {
            imported_images.insert(request.buffer_entity, gpu_image);
            Ok(if let Some(buffer) = request.buffer.take() {
                buffer
            } else {
                request
                    .client
                    .create_resource::<wl_buffer::WlBuffer, Entity, DWay>(
                        &request.display,
                        1,
                        request.buffer_entity,
                    )?
            })
        })
        .map(|buffer| {
            if request.buffer.is_none() {
                request.params.created(&buffer);
                let _ = request.display.flush_clients();
            }
            buffer
        })
        .map_err(|e| {
            error!("failed to create wl_buffer: {e}");
            request.params.failed();
        })
        .ok();
        let dma_buffer = DWayRenderResponse::ImportDmaBuffer(request.buffer_entity, buffer);
        render_server.response_tx.push(dma_buffer);
    }
}

pub struct ImportSurfacePassNode {
    surface_query: QueryState<(
        Entity,
        &'static WlSurface,
        Option<&'static WlShmBuffer>,
        Option<&'static DmaBuffer>,
        Option<&'static UninitedWlBuffer>,
    )>,
}
impl FromWorld for ImportSurfacePassNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            surface_query: QueryState::new(world),
        }
    }
}
impl Node for ImportSurfacePassNode {
    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let textures = world.resource::<RenderAssets<Image>>();
        let import_state = world.resource::<ImportState>();
        let imported_buffers = world.resource::<ImoprtedBuffers>();
        loop {
            if import_state.finished.load(Ordering::Acquire) {
                return Ok(());
            }
            if import_state
                .finished
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("import_wayland_buffer_command_encoder"),
        });
        for (entity, surface, shm_buffer, dma_buffer, egl_buffer) in
            self.surface_query.iter_manual(world)
        {
            let texture: &GpuImage = textures.get(&surface.image).unwrap();

            let result = if let Some(shm_buffer) = shm_buffer {
                match &import_state.inner {
                    Some(ImportStateKind::Egl(_)) => gles::import_shm(
                        surface,
                        shm_buffer,
                        &texture.texture,
                        render_device.wgpu_device(),
                    ),
                    Some(ImportStateKind::Vulkan(_)) => unsafe {
                        vulkan::import_shm(surface, render_queue, shm_buffer, &texture.texture)
                    },
                    None => Ok(()),
                }
            } else {
                imported_buffers
                    .get(&surface.commited.buffer.unwrap())
                    .map(|gpu_image| {
                        copy_texture(
                            &surface.commited.damages,
                            &mut command_encoder,
                            &texture.texture,
                            &gpu_image.texture,
                        )
                    })
                    .unwrap_or(Ok(()))
            };

            if let Err(e) = result {
                error!(
                    surface = %surface.raw.id(),
                    entity=?entity,
                    "failed to import buffer: {e}",
                );
            };
        }
        render_context.add_command_buffer(command_encoder.finish());
        Ok(())
    }

    fn update(&mut self, world: &mut bevy::prelude::World) {
        self.surface_query.update_archetypes(world);
    }
}

pub fn clean(
    mut wayland_map: ResMut<DWayDisplayHandles>,
    state: Res<ImportState>,
    render_device: Res<RenderDevice>,
) {
    for callback in &state.callbacks {
        debug!("emit callback {}", WlResource::id(callback));
        callback.done(state.elapsed.as_millis() as u32);
    }
    for display in wayland_map.map.values_mut() {
        let _ = display.flush_clients();
    }
    if let Some(ImportStateKind::Egl(s)) = &state.inner {
        gles::clean(s, &render_device);
    }
}

pub fn copy_texture(
    damages: &[IRect],
    command_encoder: &mut CommandEncoder,
    dest_texture: &wgpu::Texture,
    src_texture: &wgpu::Texture,
) -> Result<(), DWayRenderError> {
    let texture_extent = dest_texture.size();
    let texture_size = IVec2::new(texture_extent.width as i32, texture_extent.height as i32);
    let mut emit_rect = |rect: IRect| -> Result<()> {
        let rect = rect.intersection(IRect::from_pos_size(IVec2::ZERO, texture_size));
        debug!(?rect, "copy dma texture");
        let origin = wgpu::Origin3d {
            x: rect.x() as u32,
            y: rect.y() as u32,
            z: 0,
        };
        command_encoder.copy_texture_to_texture(
            ImageCopyTexture {
                texture: src_texture,
                mip_level: 0,
                origin,
                aspect: TextureAspect::All,
            },
            ImageCopyTexture {
                texture: dest_texture,
                mip_level: 0,
                origin,
                aspect: TextureAspect::All,
            },
            Extent3d {
                width: rect.width() as u32,
                height: rect.height() as u32,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    };

    let damage = merge_damage(damages);
    if damage.is_empty() {
        let size = dest_texture.size();
        let image_area = IRect::new(0, 0, size.width as i32, size.height as i32);
        emit_rect(image_area)?;
    } else {
        for rect in damage {
            emit_rect(rect)?;
        }
    }

    Ok(())
}

pub fn merge_damage(damage: &[IRect]) -> Vec<IRect> {
    let mut result = vec![];
    for d in damage {
        let mut merged = false;
        for r in &mut result {
            if d.union(*r).area() < d.area() + r.area() {
                *r = r.union(*d);
                merged = true;
                break;
            };
        }
        if !merged {
            result.push(*d);
        }
    }
    result
}

pub(crate) fn drm_fourcc_to_wgpu_format(
    buffer: &ImportDmaBufferRequest,
) -> Result<wgpu::TextureFormat, DWayRenderError> {
    Ok(ImageFormat::from_drm_fourcc(DrmFourcc::try_from(buffer.format)?)?.wgpu_format)
}

pub(crate) fn hal_texture_descriptor(
    size: IVec2,
    format: wgpu::TextureFormat,
) -> Result<wgpu_hal::TextureDescriptor<'static>> {
    Ok(wgpu_hal::TextureDescriptor {
        label: Some("gbm renderbuffer"),
        size: Extent3d {
            width: size.x as u32,
            height: size.y as u32,
            depth_or_array_layers: 1,
        },
        dimension: TextureDimension::D2,
        format,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUses::COLOR_TARGET
            | TextureUses::DEPTH_STENCIL_READ
            | TextureUses::DEPTH_STENCIL_WRITE
            | TextureUses::COPY_SRC
            | TextureUses::COPY_DST,
        view_formats: vec![],
        memory_flags: MemoryFlags::empty(),
    })
}

pub(crate) unsafe fn hal_texture_to_gpuimage<A>(
    device: &wgpu::Device,
    size: IVec2,
    texture_format: wgpu::TextureFormat,
    hal_texture: A::Texture,
) -> Result<GpuImage>
where
    A: hal_api::HalApi,
{
    let wgpu_texture = device.create_texture_from_hal::<A>(
        hal_texture,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[texture_format],
        },
    );
    let texture: wgpu::Texture = wgpu_texture;
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(texture_format),
        dimension: None,
        aspect: TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: Some(1.try_into().unwrap()),
        base_array_layer: 0,
        array_layer_count: None,
    });
    let sampler: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Nearest,
        mipmap_filter: FilterMode::Nearest,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
        address_mode_u: Default::default(),
        address_mode_v: Default::default(),
        address_mode_w: Default::default(),
        lod_min_clamp: Default::default(),
        lod_max_clamp: Default::default(),
    });
    Ok(GpuImage {
        texture: texture.into(),
        texture_view: texture_view.into(),
        texture_format,
        sampler: sampler.into(),
        size: size.as_vec2(),
        mip_level_count: 1,
    })
}
