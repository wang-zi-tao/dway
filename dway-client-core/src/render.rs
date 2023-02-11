use bevy::{prelude::*};

pub struct DWayRender;
impl Plugin for DWayRender{
    fn build(&self, _app: &mut App) {
        // app.add_startup_system(add_texture);
    }
}
// fn add_texture(config: &BaseRenderGraphConfig, world: &mut World) {
//   let world = world.cell();
//   // "graph" is our render graph
//   let mut graph = world.get_resource_mut::<RenderGraph>().unwrap();
//   // "msaa" contains helper methods to help define a graph given MSAA
//   let msaa = world.get_resource::<Msaa>().unwrap();
// }
