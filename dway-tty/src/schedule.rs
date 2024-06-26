use bevy::{
    prelude::*,
    render::{Render, RenderApp, RenderSet},
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

pub struct DWayTtySchedulePlugin;
impl Plugin for DWayTtySchedulePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            First,
            (
                DWayTTYSet::SeatSystem,
                DWayTTYSet::UdevSystem,
                DWayTTYSet::GbmSystem,
                DWayTTYSet::DrmSystem,
            )
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(First, DWayTTYSet::LibinputSystem);

        let render_app = app.sub_app_mut(RenderApp);
        render_app.configure_sets(
            Render,
            (
                DWayTTYRemderSet::DrmEventSystem
                    .after(RenderSet::Render)
                    .before(RenderSet::Cleanup),
                DWayTTYRemderSet::DrmCommitSystem.before(RenderSet::Cleanup),
            )
                .chain(),
        );
    }
}
