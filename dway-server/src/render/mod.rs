pub mod import;
pub mod importnode;

use bevy::{
    core_pipeline::core_2d,
    render::{
        render_graph::{RenderGraph, RunGraphOnViewNode, SlotInfo, SlotType},
        render_phase::{AddRenderCommand, DrawFunctions},
        RenderApp, RenderSet,
    },
};

use crate::prelude::*;

use self::importnode::{ImportSurface, ImportSurfacePassNode, ImportedSurfacePhaseItem};

pub struct DWayServerRenderPlugin;
impl Plugin for DWayServerRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<importnode::ImportState>();
            render_app.init_resource::<importnode::DWayDisplayHandles>();
            // render_app.init_resource::<surface::ImportSurfaceFeedback>();
            render_app.init_resource::<DrawFunctions<ImportedSurfacePhaseItem>>();
            render_app.add_render_command::<ImportedSurfacePhaseItem, ImportSurface>();
            render_app.add_system(importnode::extract_surface.in_schedule(ExtractSchedule));
            render_app.add_system(
                importnode::queue_import
                    .before(kayak_ui::render::unified::pipeline::queue_quads)
                    .in_set(RenderSet::Queue),
            );
            render_app.add_system(importnode::send_frame.in_set(RenderSet::Cleanup));
            // render_app.add_system(importnode::prepare_import_surface.in_set(RenderSet::Prepare));
            //

            let import_node = ImportSurfacePassNode::new(&mut render_app.world);
            let mut sub_graph = RenderGraph::default();
            sub_graph.add_node(importnode::node::IMPORT_PASS, import_node);
            let input_node_id = sub_graph.set_input(vec![SlotInfo::new(
                importnode::input::VIEW_ENTITY,
                SlotType::Entity,
            )]);
            sub_graph.add_slot_edge(
                input_node_id,
                importnode::input::VIEW_ENTITY,
                importnode::node::IMPORT_PASS,
                ImportSurfacePassNode::IN_VIEW,
            );

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
                graph_2d.add_slot_edge(
                    graph_2d.input_node().id,
                    core_2d::graph::input::VIEW_ENTITY,
                    importnode::node::IMPORT_PASS,
                    importnode::ImportSurfacePassNode::IN_VIEW,
                );
                // graph_2d.add_node_edge(
                //     core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                //     importnode::node::IMPORT_PASS,
                // );
                // graph_2d.add_node_edge(
                //     importnode::node::IMPORT_PASS,
                //     core_2d::graph::node::UPSCALING,
                // );
            }
        }
    }
}
