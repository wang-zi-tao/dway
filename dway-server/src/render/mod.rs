pub mod drm;
pub mod gles;
pub mod importnode;
pub mod util;
pub mod vulkan;

use std::sync::{Arc, Mutex};

use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    render::{Render, RenderApp, RenderSet, render_graph::RenderGraphExt as _},
};
use crossbeam_queue::SegQueue;
use drm::DmaBackend;
use importnode::{clean, ImoprtedBuffers};
use wayland_server::Client;

use self::importnode::ImportSurfacePassNode;
use crate::{
    prelude::*,
    zwp::dmabufparam::{DmaBuffer, DmaBufferPlane},
};
#[derive(Debug)]
pub struct ImportDmaBufferRequest {
    pub(crate) buffer_entity: Entity,
    pub(crate) buffer: Option<wl_buffer::WlBuffer>,
    pub(crate) client: Client,
    pub(crate) display: DisplayHandle,
    pub(crate) size: UVec2,
    pub(crate) format: u32,
    pub(crate) flags: WEnum<zwp_linux_buffer_params_v1::Flags>,
    pub(crate) planes: Vec<DmaBufferPlane>,
    pub(crate) params: zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
}

pub(crate) enum DWayRenderRequest {
    ImportDmaBuffer(ImportDmaBufferRequest),
}

pub enum DWayRenderResponse {
    ImportDmaBuffer(Entity, Option<wl_buffer::WlBuffer>),
}

#[derive(Resource)]
pub struct DWayServerRenderServer {
    pub(crate) response_tx: Arc<SegQueue<DWayRenderResponse>>,
    pub(crate) request_rx: Arc<SegQueue<DWayRenderRequest>>,
    pub(crate) drm_node: Arc<Mutex<Option<DmaBackend>>>,

    pub(crate) import_dma_buffer_requests: Vec<ImportDmaBufferRequest>,
}

impl DWayServerRenderServer {
    fn extract_system(mut render_server: ResMut<DWayServerRenderServer>) {
        render_server.import_dma_buffer_requests.clear();
        let mut import_dma_buffer_requests = vec![];
        while let Some(request) = render_server.request_rx.pop() {
            match request {
                DWayRenderRequest::ImportDmaBuffer(r) => {
                    import_dma_buffer_requests.push(r);
                }
            }
        }
        render_server.import_dma_buffer_requests = import_dma_buffer_requests;
    }
}

#[derive(Resource)]
pub struct DWayServerRenderClient {
    pub(crate) response_rx: Arc<SegQueue<DWayRenderResponse>>,
    pub(crate) request_tx: Arc<SegQueue<DWayRenderRequest>>,
    pub(crate) drm_node: Arc<Mutex<Option<DmaBackend>>>,
}

impl DWayServerRenderClient {
    pub(crate) fn new() -> (DWayServerRenderClient, DWayServerRenderServer) {
        let request_queue = Arc::new(SegQueue::new());
        let response_queue = Arc::new(SegQueue::new());
        let drm_node_cell = Arc::new(Mutex::new(None));
        (
            DWayServerRenderClient {
                response_rx: response_queue.clone(),
                request_tx: request_queue.clone(),
                drm_node: drm_node_cell.clone(),
            },
            DWayServerRenderServer {
                response_tx: response_queue,
                request_rx: request_queue,
                drm_node: drm_node_cell,
                import_dma_buffer_requests: vec![],
            },
        )
    }

    fn response_system(render_client: Res<DWayServerRenderClient>, mut commands: Commands) {
        while let Some(response) = render_client.response_rx.pop() {
            match response {
                DWayRenderResponse::ImportDmaBuffer(entity, buffer) => {
                    commands.queue(move |world: &mut World| {
                        let Some(buffer) = buffer else {
                            if let Ok(e) = world.get_entity_mut(entity) {
                                e.despawn();
                            }
                            return;
                        };
                        let Some(mut dma_buffer) = world.get_mut::<DmaBuffer>(entity) else {
                            return;
                        };
                        debug!(entity=?entity, dma_buffer=?buffer.id(), "insert dma buffer");
                        dma_buffer.raw = Some(buffer)
                    });
                }
            }
        }
    }
}

pub struct DWayServerRenderPlugin;
impl Plugin for DWayServerRenderPlugin {
    fn build(&self, app: &mut App) {
        let (client, server) = DWayServerRenderClient::new();
        app.insert_resource(client);
        app.add_systems(
            PreUpdate,
            DWayServerRenderClient::response_system.before(DWayServerSet::Dispatch),
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(server);
            render_app.init_resource::<ImoprtedBuffers>();

            render_app.init_resource::<importnode::ImportState>();
            render_app.init_resource::<importnode::DWayDisplayHandles>();
            render_app.add_systems(ExtractSchedule, importnode::extract_surface);
            render_app.add_systems(
                Render,
                DWayServerRenderServer::extract_system.in_set(RenderSet::PrepareAssets),
            );
            render_app.add_systems(
                Render,
                importnode::prepare_surfaces.after(RenderSet::PrepareAssets),
            );

            render_app
                .add_render_graph_node::<ImportSurfacePassNode>(
                    Core2d,
                    importnode::graph::Labels2d::Import,
                )
                .add_render_graph_edges(
                    Core2d,
                    (Node2d::MsaaWriteback, importnode::graph::Labels2d::Import, Node2d::StartMainPass),
                );

            render_app.add_systems(
                Render,
                drm::init_drm_state
                    .run_if(|s: Res<DWayServerRenderServer>| s.is_added())
                    .in_set(RenderSet::Prepare),
            );
            render_app.add_systems(
                Render,
                clean
                    .after(RenderSet::Render)
                    .before(RenderSet::Cleanup),
            );
        }
    }
}
