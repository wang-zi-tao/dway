pub mod dmabuffactory;
pub mod dmabuffeedback;
pub mod dmabufparam;
pub mod idle;

use crate::prelude::*;
use self::{
    dmabuffeedback::DmabufFeedback,
    dmabufparam::DmaBuffer,
};

pub struct DmaBufferPlugin;
impl Plugin for DmaBufferPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(dmabuffactory::DWayDmaBufferFactoryPlugin);
        app.register_type::<DmaBuffer>();
        app.register_type::<DmabufFeedback>();
    }
}
