use bevy::ecs::schedule::{FreeSystemSet, ScheduleLabel};

use crate::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayStartSet {
    CreateDisplay,
    Flush,
    Spawn,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayServerSet {
    Create,
    CreateGlobal,
    Dispatch,

    UpdateXWayland,
    UpdateGeometry,

    UpdateJoin,
    Update,
    PostUpdate,
    PostUpdateFlush,
    Last,
    LastFlush,

    Input,
    GrabInput,
    InputFlush,
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
            (
                DWayStartSet::CreateDisplay,
                DWayStartSet::Flush,
                DWayStartSet::Spawn,
            )
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (
                UpdateGeometry,
                UpdateXWayland,
                UpdateJoin.after(UpdateGeometry),
            )
                .after(Dispatch)
                .before(Update)
                .in_base_set(CoreSet::PreUpdate),
        );
        app.configure_sets(
            (Create, CreateGlobal, Dispatch, UpdateJoin, Update)
                .chain()
                .in_base_set(CoreSet::PreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (PostUpdate, PostUpdateFlush)
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            (Input, GrabInput, InputFlush)
                .chain()
                .in_base_set(CoreSet::PreUpdate)
                .before(Create),
        );

        app.configure_sets(
            (Last, LastFlush)
                .chain()
                .in_base_set(CoreSet::Last)
                .ambiguous_with_all(),
        );
    }
}
