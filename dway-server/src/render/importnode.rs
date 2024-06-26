use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

use super::{
    gles::EglState,
    util::DWayRenderError,
    vulkan::{self, VulkanState},
};
use crate::{
    prelude::*,
    state::{WaylandDisplayCreated, WaylandDisplayDestroyed},
    util::rect::IRect,
    wl::{
        buffer::{UninitedWlBuffer, WlShmBuffer},
        surface::WlSurface,
    },
    zwp::dmabufparam::DmaBuffer,
};
use bevy::{
    core::FrameCount,
    ecs::{entity::EntityHashMap, system::SystemState},
    render::{
        render_asset::RenderAssets,
        render_graph::Node,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract,
    },
    utils::HashSet,
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
    Vulkan(crate::render::vulkan::ImportedImage),
}

#[derive(Resource, Default)]
pub struct ImportState {
    pub inner: Mutex<Option<ImportStateKind>>,
    pub removed_image: Vec<AssetId<Image>>,
    pub image_set: HashSet<Handle<Image>>,
    pub finished: AtomicBool,
}

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

pub fn extract_surface(
    surface_query: Extract<Query<&WlSurface>>,
    shm_buffer_query: Extract<Query<&WlShmBuffer>>,
    dma_buffer_query: Extract<Query<&DmaBuffer>>,
    egl_buffer_query: Extract<Query<&UninitedWlBuffer>>,
    mut commands: Commands,
    frame_count: Extract<Res<FrameCount>>,
    mut create_events: Extract<EventReader<WaylandDisplayCreated>>,
    mut destroy_events: Extract<EventReader<WaylandDisplayDestroyed>>,
    mut wayland_map: ResMut<DWayDisplayHandles>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
    mut state: ResMut<ImportState>,
) {
    state.removed_image.clear();
    state.removed_image.clear();
    for surface in surface_query.iter() {
        if !(surface.just_commit
            || surface.commit_time + 2 >= frame_count.0 && surface.commit_count <= 2)
        {
            continue;
        }
        let Some(buffer_entity) = surface.commited.buffer else {
            debug!("surface {:?} has no attachment", surface.raw.id());
            continue;
        };
        let _span = debug_span!("extract surface",buffer=?buffer_entity).entered();

        if let Ok(buffer) = shm_buffer_query.get(buffer_entity) {
            debug!("use shared memory buffer");
            commands.spawn((surface.clone(), buffer.clone()));
        } else if let Ok(dma_buffer) = dma_buffer_query.get(buffer_entity) {
            debug!("use dma buffer");
            commands.spawn((surface.clone(), dma_buffer.clone()));
        } else if let Ok(egl_buffer) = egl_buffer_query.get(buffer_entity) {
            debug!("use egl buffer");
            commands.spawn((surface.clone(), egl_buffer.clone()));
        } else {
            error!(entity=?buffer_entity,"buffer not found");
        };
        debug!("extract wayland buffer: {buffer_entity:?}");
        state.image_set.insert(surface.image.clone_weak());
    }
    for WaylandDisplayCreated(entity, display_handle) in create_events.read() {
        wayland_map.map.insert(*entity, display_handle.clone());
    }
    for WaylandDisplayDestroyed(entity, _display_handle) in destroy_events.read() {
        wayland_map.map.remove(entity);
    }
    for event in image_events.read() {
        match event {
            AssetEvent::Removed { id } => state.removed_image.push(*id),
            _ => {}
        }
    }
}

#[tracing::instrument(skip_all)]
pub fn prepare_surfaces(
    surface_query: Query<(&WlSurface, Option<&WlShmBuffer>, Option<&DmaBuffer>)>,
    render_device: Res<RenderDevice>,
    import_state: ResMut<ImportState>,
    mut images: ResMut<RenderAssets<Image>>,
) {
    import_state.finished.store(false, Ordering::Relaxed);
    let mut state_guard = import_state.inner.lock().unwrap();
    if state_guard.is_none() {
        match ImportStateKind::new(render_device.wgpu_device()) {
            Ok(o) => *state_guard = Some(o),
            Err(e) => {
                error!("failed to prepare wayland surface: {e}");
            }
        }
    }
    let Some(state_guard) = state_guard.as_mut() else {
        return;
    };
    match state_guard{
        ImportStateKind::Egl(state) => {},
        ImportStateKind::Vulkan(state) => {
            state.shm_image_map.retain(|k,_|k.is_alive());
            state.dma_image_map.retain(|k,_|k.is_alive());
        },
    };
    surface_query
        .iter()
        .for_each(|(surface, shm_buffer, dma_buffer)| {
            match state_guard {
                ImportStateKind::Egl(_) => {}
                ImportStateKind::Vulkan(vulkan) => {
                    let _span = debug_span!("prepare wayland surface", surface = ?surface.raw.id()).entered();
                    if let Err(e) = vulkan::prepare_wl_surface(
                        vulkan,
                        render_device.wgpu_device(),
                        surface,
                        shm_buffer,
                        dma_buffer,
                        &mut images,
                    ) {
                        error!("failed to prepare wayland surface: {e}");
                    };
                }
            };
        });
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
        _render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let textures = world.resource::<RenderAssets<Image>>();
        let import_state = world.resource::<ImportState>();
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
        for (entity, surface, buffer, dma_buffer, egl_buffer) in
            self.surface_query.iter_manual(world)
        {
            let texture: &GpuImage = textures.get(&surface.image).unwrap();
            let mut state = import_state.inner.lock().unwrap();
            let result = match &mut *state {
                Some(ImportStateKind::Egl(gles)) => super::gles::import_wl_surface(
                    surface,
                    buffer,
                    dma_buffer,
                    egl_buffer,
                    &texture.texture,
                    render_device.wgpu_device(),
                    gles,
                ),
                Some(ImportStateKind::Vulkan(_)) => {
                    super::vulkan::import_wl_surface(surface, buffer, &texture.texture, &render_queue)
                }
                None => continue,
            };

            if let Err(e) = result {
                error!(
                    surface = %surface.raw.id(),
                    entity=?entity,
                    "failed to import buffer: {e}",
                );
            } else {
                trace!(
                    surface = %surface.raw.id(),
                    entity=?entity,
                    "import buffer",
                );
            };
        }

        Ok(())
    }

    fn update(&mut self, world: &mut bevy::prelude::World) {
        self.surface_query.update_archetypes(world);
    }
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
