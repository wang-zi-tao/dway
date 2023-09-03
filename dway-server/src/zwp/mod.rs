pub mod dambuffeedback;
pub mod dmabuffactory;
pub mod dmabufparam;
use crate::{prelude::*, render::drm::DrmNodeState, schedule::DWayServerSet};

use self::{
    dambuffeedback::{do_init_feedback, update_feedback_state, DmabufFeedback},
    dmabufparam::DmaBuffer,
};

pub struct DmaBufferPlugin;
impl Plugin for DmaBufferPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(dmabuffactory::DWayDmaBufferFactoryPlugin);
        app.register_type::<DmaBuffer>();
        app.register_type::<DmabufFeedback>();
        app.add_system(update_feedback_state.in_set(DWayServerSet::InitDmaBufFeedback));
    }
}
