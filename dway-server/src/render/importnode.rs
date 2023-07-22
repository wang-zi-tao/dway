use crate::{
    prelude::*,
    render::import::import_wl_surface,
    wl::{
        buffer::{DmaBuffer, EGLBuffer, WlBuffer, WlShmPool},
        surface::WlSurface,
    },
};

use std::{
    cell::{RefCell, RefMut},
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use bevy::{
    core::FrameCount,
    core_pipeline::{clear_color::ClearColorConfig, core_2d::Transparent2d},
    ecs::system::lifetimeless::{Read, SRes, SResMut},
    log::Level,
    prelude::*,
    render::{
        camera::ExtractedCamera,
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph, SlotInfo, SlotType},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase},
        renderer::{RenderAdapter, RenderDevice, RenderQueue},
        texture::GpuImage,
        view::{ExtractedView, NonSendMarker, ViewTarget},
        Extract,
    },
    sprite::SpriteAssetEvents,
    ui::UiImageBindGroups,
    utils::{
        tracing::{self, span},
        HashSet,
    },
};
use failure::Fallible;
use glow::HasContext;
use wgpu::{
    Extent3d, LoadOp, Operations, RenderPass, RenderPassDescriptor, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

pub struct ImportedSurfacePhaseItem {
    pub entity: Entity,
    pub draw_function: DrawFunctionId,
}
impl PhaseItem for ImportedSurfacePhaseItem {
    type SortKey = u32;

    fn entity(&self) -> bevy::prelude::Entity {
        self.entity
    }

    fn sort_key(&self) -> Self::SortKey {
        1
    }

    fn draw_function(&self) -> bevy::render::render_phase::DrawFunctionId {
        self.draw_function
    }
}

pub fn extract_surface(
    _: NonSend<NonSendMarker>,
    surface_query: Extract<Query<&WlSurface>>,
    buffer_query: Extract<Query<(&WlBuffer, &Parent, Option<&DmaBuffer>, Option<&EGLBuffer>)>>,
    shm_pool_query: Extract<Query<&WlShmPool>>,
    mut commands: Commands,
    mut image_bind_groups: Option<ResMut<kayak_ui::render::unified::pipeline::ImageBindGroups>>,
    frame_count: Extract<Res<FrameCount>>,
) {
    for surface in surface_query.iter() {
        if !surface.just_commit && surface.commit_time + 2 < frame_count.0 {
            continue;
        }
        if let Some(image_bind_groups) = image_bind_groups.as_mut() {
            // debug!("remove bind group of {:?}", &surface.image);
            image_bind_groups.values.remove(&surface.image);
        }
        let Some(buffer_entity) = surface.commited.buffer else {
            trace!("no wl_buffer {:?}", surface.raw.id());
            continue;
        };
        let Ok((buffer, shm_pool_entity, dma_buffer, egl_buffer)) = buffer_query.get(buffer_entity)
        else {
            trace!("no wl_shm_pool {:?}", surface.raw.id());
            continue;
        };
        let Ok(shm_pool) = shm_pool_query.get(shm_pool_entity.get()) else {
            trace!("no shm_pool_query {:?}", surface.raw.id());
            continue;
        };
        // trace!("extract {:?}", surface.raw.id());
        let mut entity = commands.spawn((surface.clone(), buffer.clone(), shm_pool.clone()));
        if let Some(dma_buffer) = dma_buffer {
            entity.insert(dma_buffer.clone());
        }
        if let Some(egl_buffer) = egl_buffer {
            entity.insert(egl_buffer.clone());
        }
    }
    commands.spawn(RenderPhase::<ImportedSurfacePhaseItem>::default());
}

pub fn queue_import(
    draw_functions: Res<DrawFunctions<ImportedSurfacePhaseItem>>,
    mut phase_query: Query<&mut RenderPhase<ImportedSurfacePhaseItem>>,
    surface_query: Query<Entity, With<WlSurface>>,
) {
    let function = draw_functions.read().id::<ImportSurface>();
    let mut phase = phase_query.single_mut();
    for entity in &surface_query {
        phase.add(ImportedSurfacePhaseItem {
            draw_function: function,
            entity,
        });
    }
}
pub fn send_frame(
    _: NonSend<NonSendMarker>,
    time: Res<Time>,
    // feedback: ResMut<ImportSurfaceFeedback>,
) {
    // feedback.send_frame(&time);
    // trace!(thread=?thread::current().id(),"send frame");
}

pub struct ImportSurface;
impl<P: PhaseItem> RenderCommand<P> for ImportSurface {
    type Param = (SRes<RenderDevice>, SRes<RenderAssets<Image>>);
    type ItemWorldQuery = (
        Read<WlSurface>,
        Read<WlBuffer>,
        Read<WlShmPool>,
        Option<Read<DmaBuffer>>,
        Option<Read<EGLBuffer>>,
    );
    type ViewWorldQuery = ();

    fn render<'w>(
        item: &P,
        view: bevy::ecs::query::ROQueryItem<'w, Self::ViewWorldQuery>,
        (surface, buffer, shm_pool, dma_buffer, egl_buffer): bevy::ecs::query::ROQueryItem<
            'w,
            Self::ItemWorldQuery,
        >,
        (render_device, textures): bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        let texture: &GpuImage = textures.get(&surface.image).unwrap();
        if let Err(e) = import_wl_surface(
            surface,
            buffer,
            shm_pool,
            dma_buffer,
            egl_buffer,
            &texture.texture,
            render_device.wgpu_device(),
        ) {
            error!(
                surface = ?surface.raw.id(),
                error = %e,
                entity=?item.entity(),
                texture = ?&texture.texture,
                "failed to import buffer.",
            );
            return bevy::render::render_phase::RenderCommandResult::Success;
        } else {
            trace!(
                surface = ?surface.raw.id(),
                entity=?item.entity(),
                texture = ?&texture.texture,
                "import buffer",
            );
        };
        bevy::render::render_phase::RenderCommandResult::Success
    }
}

pub struct ImportSurfacePassNode {
    query: QueryState<(Entity, &'static RenderPhase<ImportedSurfacePhaseItem>)>,
    view_query: QueryState<
        (
            &'static ExtractedCamera,
            &'static ViewTarget,
            &'static Camera2d,
        ),
        With<ExtractedView>,
    >,
}
impl ImportSurfacePassNode {
    pub const IN_VIEW: &'static str = "view";
    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query(),
            view_query: world.query_filtered(),
        }
    }
}
impl Node for ImportSurfacePassNode {
    fn run(
        &self,
        graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, target, camera_2d) =
            if let Ok(result) = self.view_query.get_manual(world, view_entity) {
                result
            } else {
                return Ok(());
            };
        {
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("import_wayland_buffer"),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: match camera_2d.clear_color {
                        ClearColorConfig::Default => {
                            LoadOp::Clear(world.resource::<ClearColor>().0.into())
                        }
                        ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                        ClearColorConfig::None => LoadOp::Load,
                    },
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            for (entity, phase) in self.query.iter_manual(world) {
                phase.render(&mut render_pass, world, entity);
            }
        }

        Ok(())
    }

    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut bevy::prelude::World) {
        self.query.update_archetypes(world);
        self.view_query.update_archetypes(world);
    }
}
pub const NAME: &'static str = "wayland_server_graph";
pub mod node {
    pub const IMPORT_PASS: &'static str = "import_wayland_surface";
}
pub mod input {
    pub const VIEW_ENTITY: &'static str = "view_entity";
}
