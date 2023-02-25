use bevy::{
    prelude::*,
    render::{
        renderer::{RenderDevice, RenderQueue},
        RenderApp,
    },
};

pub struct DWayRender;
impl Plugin for DWayRender {
    fn build(&self, app: &mut App) {
        let render_app: &mut App = app.sub_app_mut(RenderApp);
        // app.add_startup_system(add_texture);
    }
}

pub fn prepare(render_device: Res<RenderDevice>, render_queue: Res<RenderQueue>) {}
// fn add_texture(config: &BaseRenderGraphConfig, world: &mut World) {
//   let world = world.cell();
//   // "graph" is our render graph
//   let mut graph = world.get_resource_mut::<RenderGraph>().unwrap();
//   // "msaa" contains helper methods to help define a graph given MSAA
//   let msaa = world.get_resource::<Msaa>().unwrap();
// }
