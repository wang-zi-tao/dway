pub mod dmabuffactory;
pub mod dambuffeedback;
pub mod dmabufparam;
use crate::prelude::*;

pub struct DmaBufferPlugin;
impl Plugin for DmaBufferPlugin{
    fn build(&self, app: &mut App) {
        app.add_plugin(dmabuffactory::DWayDmaBufferFactoryPlugin);
    }
}
