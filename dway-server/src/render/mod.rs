pub mod import;
pub mod importnode;

use bevy::{render::{RenderApp, render_phase::{DrawFunctions, AddRenderCommand}, render_graph::RenderGraph, RenderSet}, core_pipeline::core_2d, ui::draw_ui_graph};

use crate::{prelude::*, wl::surface::WlSurface};

use self::importnode::{ImportedSurfacePhaseItem, ImportSurfacePassNode, ImportSurface}; 

pub struct DWayServerRenderPlugin;
impl Plugin for DWayServerRenderPlugin{
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            // render_app.init_resource::<surface::ImportSurfaceFeedback>();
            render_app.init_resource::<DrawFunctions<ImportedSurfacePhaseItem>>();
            render_app
                .add_render_command::<ImportedSurfacePhaseItem, ImportSurface>();
            render_app.add_system(importnode::extract_surface.in_schedule(ExtractSchedule));
            render_app.add_system(importnode::queue_import.in_set(RenderSet::Queue));
            render_app.add_system(importnode::send_frame.in_set(RenderSet::Cleanup));
            // render_app.add_system(importnode::prepare_import_surface.in_set(RenderSet::Prepare));
            //

            let import_node = ImportSurfacePassNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            if let Some(graph_2d) =
                graph.get_sub_graph_mut(bevy::core_pipeline::core_2d::graph::NAME)
            {
                graph_2d.add_node(importnode::node::NAME, import_node);
                // graph_2d.add_node_edge(core_2d::graph::node::MAIN_PASS, surface::node::NAME);
                graph_2d.add_slot_edge(
                    graph_2d.input_node().id,
                    core_2d::graph::input::VIEW_ENTITY,
                    importnode::node::NAME,
                    importnode::ImportSurfacePassNode::IN_VIEW,
                );
                graph_2d.add_node_edge(importnode::node::NAME, draw_ui_graph::node::UI_PASS);
            }
        }
    }
}
