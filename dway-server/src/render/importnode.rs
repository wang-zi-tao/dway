use std::{collections::HashMap, sync::Mutex};

use crate::{
    prelude::*,
    render::import::import_wl_surface,
    schedule::DWayServerSet,
    state::{WaylandDisplayCreated, WaylandDisplayDestroyed},
    wl::{
        buffer::{DmaBuffer, EGLBuffer, WlBuffer},
        surface::WlSurface,
    },
};

use bevy::{
    core::FrameCount,
    core_pipeline::clear_color::ClearColorConfig,
    ecs::system::{
        lifetimeless::{Read, SRes},
        SystemParam, SystemState,
    },
    render::{
        camera::ExtractedCamera,
        render_asset::RenderAssets,
        render_graph::{Node, SlotInfo, SlotType},
        render_phase::{DrawFunctionId, DrawFunctions, PhaseItem, RenderCommand, RenderPhase},
        renderer::RenderDevice,
        texture::GpuImage,
        view::{ExtractedView, NonSendMarker, ViewTarget},
        Extract,
    },
};

use wgpu::{LoadOp, Operations, RenderPassDescriptor};

use super::import::{bind_wayland, EglState};

#[derive(Resource, Default)]
pub struct ImportState {
    pub inner: Mutex<Option<EglState>>,
}
#[derive(Resource, Default)]
pub struct DWayDisplayHandles {
    pub map: HashMap<Entity, DisplayHandle>,
}

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
    mut commands: Commands,
    mut image_bind_groups: Option<ResMut<kayak_ui::render::unified::pipeline::ImageBindGroups>>,
    frame_count: Extract<Res<FrameCount>>,
    mut create_events: Extract<EventReader<WaylandDisplayCreated>>,
    mut destroy_events: Extract<EventReader<WaylandDisplayDestroyed>>,
    mut wayland_map: ResMut<DWayDisplayHandles>,
) {
    for surface in surface_query.iter() {
        if !(surface.just_commit
            || surface.commit_time + 2 >= frame_count.0 && surface.commit_count <= 2)
        {
            continue;
        }
        if let Some(image_bind_groups) = image_bind_groups.as_mut() {
            // debug!("remove bind group of {:?}", &surface.image);
            image_bind_groups.values.remove(&surface.image);
        }
        let Some(buffer_entity) = surface.commited.buffer else {
            trace!("not connited {:?}", surface.raw.id());
            continue;
        };
        let Ok((buffer, _shm_pool_entity, dma_buffer, egl_buffer)) =
            buffer_query.get(buffer_entity)
        else {
            trace!("no wl_buffer {:?}", buffer_entity);
            continue;
        };
        // trace!("extract {:?}", surface.raw.id());
        let mut entity = commands.spawn((surface.clone(), buffer.clone()));
        if let Some(dma_buffer) = dma_buffer {
            entity.insert(dma_buffer.clone());
        }
        if let Some(egl_buffer) = egl_buffer {
            entity.insert(egl_buffer.clone());
        }
    }
    commands.spawn(RenderPhase::<ImportedSurfacePhaseItem>::default());
    for WaylandDisplayCreated(entity, display_handle) in create_events.iter() {
        wayland_map.map.insert(*entity, display_handle.clone());
    }
    for WaylandDisplayDestroyed(entity, display_handle) in destroy_events.iter() {
        wayland_map.map.remove(entity);
    }
}

pub fn queue_import(
    draw_functions: Res<DrawFunctions<ImportedSurfacePhaseItem>>,
    mut phase_query: Query<&mut RenderPhase<ImportedSurfacePhaseItem>>,
    surface_query: Query<Entity, With<WlSurface>>,
    display_handles: Res<DWayDisplayHandles>,
    import_state: Res<ImportState>,
    render_device: Res<RenderDevice>,
) {
    let function = draw_functions.read().id::<ImportSurface>();
    let mut phase = phase_query.single_mut();
    for entity in &surface_query {
        phase.add(ImportedSurfacePhaseItem {
            draw_function: function,
            entity,
        });
    }

    let mut state = import_state.inner.lock().unwrap();
    if let Some(mut state) = state.as_mut() {
        if let Err(e) = bind_wayland(&display_handles, &mut state, render_device.wgpu_device()) {
            error!("{e}");
        };
    }
}
pub fn send_frame(
    _: NonSend<NonSendMarker>,
    _time: Res<Time>,
    // feedback: ResMut<ImportSurfaceFeedback>,
) {
    // feedback.send_frame(&time);
    // trace!(thread=?thread::current().id(),"send frame");
}

pub struct ImportSurface;
impl<P: PhaseItem> RenderCommand<P> for ImportSurface {
    type Param = (
        SRes<RenderDevice>,
        SRes<RenderAssets<Image>>,
        SRes<ImportState>,
    );
    type ItemWorldQuery = (
        Read<WlSurface>,
        Read<WlBuffer>,
        Option<Read<DmaBuffer>>,
        Option<Read<EGLBuffer>>,
    );
    type ViewWorldQuery = ();

    fn render<'w>(
        item: &P,
        _view: bevy::ecs::query::ROQueryItem<'w, Self::ViewWorldQuery>,
        (surface, buffer, dma_buffer, egl_buffer): bevy::ecs::query::ROQueryItem<
            'w,
            Self::ItemWorldQuery,
        >,
        (render_device, textures, import_state): bevy::ecs::system::SystemParamItem<
            'w,
            '_,
            Self::Param,
        >,
        _pass: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        let texture: &GpuImage = textures.get(&surface.image).unwrap();
        let mut state_guard = import_state.inner.lock().unwrap();
        if let Err(e) = import_wl_surface(
            surface,
            buffer,
            dma_buffer,
            egl_buffer,
            &texture.texture,
            render_device.wgpu_device(),
            &mut state_guard,
        ) {
            error!(
                surface = %surface.raw.id(),
                error = %e,
                entity=?item.entity(),
                texture = ?&texture.texture,
                "failed to import buffer.",
            );
            return bevy::render::render_phase::RenderCommandResult::Success;
        } else {
            trace!(
                surface = %surface.raw.id(),
                entity=?item.entity(),
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

            let device = world.resource::<RenderDevice>();
            let import_state = world.resource::<ImportState>();
            let display_handles = world.resource::<DWayDisplayHandles>();
            let mut state_guard = import_state.inner.lock().unwrap();
            if let Some(state_guard) = state_guard.as_mut() {
                let egl_display: khronos_egl::Display = unsafe {
                    device
                        .wgpu_device()
                        .as_hal::<wgpu_hal::api::Gles, _, _>(|hal_device| {
                            hal_device
                                .ok_or_else(|| anyhow!("gpu backend is not egl"))?
                                .context()
                                .raw_display()
                                .cloned()
                                .ok_or_else(|| anyhow!("no opengl display available"))
                        })
                        .unwrap()
                };
                state_guard.bind_wayland(&display_handles.map, egl_display);
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
pub const NAME: &str = "wayland_server_graph";
pub mod node {
    pub const IMPORT_PASS: &str = "import_wayland_surface";
}
pub mod input {
    pub const VIEW_ENTITY: &str = "view_entity";
}
