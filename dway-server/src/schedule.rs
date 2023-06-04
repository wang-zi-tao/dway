use bevy::ecs::schedule::{FreeSystemSet, ScheduleLabel};

use crate::prelude::*;

#[derive(SystemSet,Debug,Clone,PartialEq, Eq,Hash)]
pub enum DWayServerSet{
    Dispatch,
    UpdateGeometry,
    UpdateJoin,
    Update,
    PostUpdate,
}
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DWayServerSchedule{
    StartUp,
    Main,
    Outer,
    FixedUpdate,
}
pub struct DWayServerSchedulePlugin;
impl Plugin for DWayServerSchedulePlugin{
    fn build(&self, app: &mut App) {
        use DWayServerSet::*;
        app.configure_sets(
            (
                Dispatch,
                UpdateGeometry,
                UpdateJoin,
                Update,
            )
                .chain()
                .in_base_set(CoreSet::PreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (PostUpdate, )
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .ambiguous_with_all(),
        );
    }
}
