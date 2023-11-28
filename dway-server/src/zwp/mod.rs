pub mod dmabuffactory;
pub mod dmabuffeedback;
pub mod dmabufparam;
pub mod idle;

use self::{dmabuffeedback::DmabufFeedback, dmabufparam::DmaBuffer};
use crate::prelude::*;

pub struct DmaBufferPlugin;
impl Plugin for DmaBufferPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(dmabuffactory::DWayDmaBufferFactoryPlugin);
        app.register_type::<DmaBuffer>();
        app.register_type::<DmabufFeedback>();
    }
}
