pub mod dmabuffactory;
pub mod dambuffeedback;
pub mod dmabufparam;
use crate::prelude::*;

use self::{dmabufparam::DmaBuffer, dambuffeedback::DmabufFeedback};

pub struct DmaBufferPlugin;
impl Plugin for DmaBufferPlugin{
    fn build(&self, app: &mut App) {
        app.add_plugin(dmabuffactory::DWayDmaBufferFactoryPlugin);
        app.register_type::<DmaBuffer>();
        app.register_type::<DmabufFeedback>();
    }
}
