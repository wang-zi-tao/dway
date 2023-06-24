use bevy::ecs::schedule::{FreeSystemSet, ScheduleLabel};

use crate::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayStartSet{
    CreateDisplay,
    Flush,
    Spawn,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayServerSet {
    Create,
    CreateGlobal,
    Dispatch,
    UpdateGeometry,
    UpdateJoin,
    Update,
    PostUpdate,
}
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DWayServerSchedule {
    StartUp,
    Main,
    Outer,
    FixedUpdate,
}
pub struct DWayServerSchedulePlugin;
impl Plugin for DWayServerSchedulePlugin {
    fn build(&self, app: &mut App) {
        use DWayServerSet::*;
        app.configure_sets(
            (DWayStartSet::CreateDisplay,DWayStartSet::Flush,DWayStartSet::Spawn)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (Create,CreateGlobal,Dispatch, UpdateGeometry, UpdateJoin, Update)
                .chain()
                .in_base_set(CoreSet::PreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (PostUpdate,)
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .ambiguous_with_all(),
        );
    }
}
