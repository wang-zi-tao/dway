pub mod drm;
pub mod gles;
pub mod importnode;
pub mod util;
pub mod vulkan;

use bevy::{
    core_pipeline::core_2d::{self, CORE_2D},
    render::{
        render_asset::prepare_assets,
        render_graph::{RenderGraphApp},
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

            render_app
                .add_render_graph_node::<ImportSurfacePassNode>(
                    CORE_2D,
                    ImportSurfacePassNode::NAME,
                )
                .add_render_graph_edges(
                    CORE_2D,
                    &[core_2d::graph::node::MAIN_PASS, ImportSurfacePassNode::NAME],
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
