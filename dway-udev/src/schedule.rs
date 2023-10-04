use bevy::{
    prelude::*,
    render::{RenderApp, RenderSet},
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayTTYSet {
    SeatSystem,
    UdevSystem,
    GbmSystem,
    DrmSystem,
    DrmEventSystem,
    LibinputSystem,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayTTYRemderSet {
    DrmEventSystem,
    DrmCommitSystem,
}

pub struct DWayUdevSchedulePlugin;
impl Plugin for DWayUdevSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            (
                DWayTTYSet::SeatSystem,
                DWayTTYSet::UdevSystem,
                DWayTTYSet::GbmSystem,
                DWayTTYSet::DrmSystem,
            )
                .in_base_set(CoreSet::First)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_set(DWayTTYSet::LibinputSystem.in_base_set(CoreSet::First));

        let render_app = app.sub_app_mut(RenderApp);
        render_app.configure_sets(
            (
                DWayTTYRemderSet::DrmEventSystem.after(RenderSet::Render),
                DWayTTYRemderSet::DrmCommitSystem.before(RenderSet::Cleanup),
            )
                .chain(),
        );
    }
}
