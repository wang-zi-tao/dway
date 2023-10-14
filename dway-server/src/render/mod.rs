pub mod drm;
pub mod gles;
pub mod importnode;
pub mod util;
pub mod vulkan;

use bevy::{
    core_pipeline::core_2d,
    render::{
        render_asset::prepare_assets,
        render_graph::{RenderGraph, RunGraphOnViewNode},
        Render, RenderApp, RenderSet,
    },
};

use crate::{prelude::*, schedule::DWayServerSet};

use self::{drm::DrmNodeState, importnode::ImportSurfacePassNode};

pub struct DWayServerRenderPlugin;
impl Plugin for DWayServerRenderPlugin {
    fn build(&self, app: &mut App) {
        let (feedback, drm_state) = drm::new_drm_node_resource();
        app.insert_resource(feedback);
        app.add_systems(
            PreUpdate,
            drm::update_dma_feedback_writer.in_set(DWayServerSet::CreateGlobal),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(drm_state);

            render_app.init_resource::<importnode::ImportState>();
            render_app.init_resource::<importnode::DWayDisplayHandles>();
            render_app.add_systems(ExtractSchedule, importnode::extract_surface);
            render_app.add_systems(
                Render,
                importnode::prepare_surfaces.after(prepare_assets::<Image>),
            );

            let import_node = ImportSurfacePassNode::new(&mut render_app.world);
            let mut sub_graph = RenderGraph::default();
            sub_graph.add_node(importnode::node::IMPORT_PASS, import_node);

            let mut graph = render_app.world.resource_mut::<RenderGraph>();

            if let Some(graph_2d) =
                graph.get_sub_graph_mut(bevy::core_pipeline::core_2d::graph::NAME)
            {
                graph_2d.add_sub_graph(importnode::NAME, sub_graph);
                graph_2d.add_node(
                    importnode::node::IMPORT_PASS,
                    RunGraphOnViewNode::new(importnode::NAME),
                );
                graph_2d.add_node_edge(
                    core_2d::graph::node::MAIN_PASS,
                    importnode::node::IMPORT_PASS,
                );
            }

            render_app.add_systems(
                Render,
                drm::init_drm_state
                    .run_if(|s: Res<DrmNodeState>| s.state.is_none())
                    .in_set(RenderSet::Prepare),
            );
        }
    }
}
