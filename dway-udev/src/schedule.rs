use bevy::ecs::schedule::ScheduleLabel;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayTTYSet {
    SeatSystem,
    UdevSystem,
    GbmSystem,
    DrmSystem,
    LibinputSystem,
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
    }
}
