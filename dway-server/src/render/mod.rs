pub mod drm;
pub mod gles;
pub mod importnode;
pub mod util;
pub mod vulkan;

use crate::prelude::*;
use bevy::{
    core_pipeline::core_2d::graph::{Core2d, Node2d},
    render::{
        render_asset::prepare_assets, render_graph::RenderGraphApp, Render, RenderApp, RenderSet,
    },
};

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
                importnode::prepare_surfaces
                    .in_set(RenderSet::PrepareAssets)
                    .after(prepare_assets::<Image>),
            );

            render_app
                .add_render_graph_node::<ImportSurfacePassNode>(
                    Core2d,
                    importnode::graph::Labels2d::Import,
                )
                .add_render_graph_edges(
                    Core2d,
                    (Node2d::MainPass, importnode::graph::Labels2d::Import),
                );

            render_app.add_systems(
                Render,
                drm::init_drm_state
                    .run_if(|s: Res<DrmNodeState>| s.state.is_none())
                    .in_set(RenderSet::Prepare),
            );
        }
    }
}
