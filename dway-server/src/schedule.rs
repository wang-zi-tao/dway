use bevy::ecs::schedule::ScheduleLabel;

use crate::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayStartSet {
    CreateDisplay,
    CreateDisplayFlush,
    Spawn,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayServerSet {
    Create,
    CreateGlobal,
    Dispatch,

    InitDmaBufFeedback,
    UpdateXWayland,
    UpdateGeometry,
    UpdateSurface,
    UpdateAppInfo,

    Input,
    GrabInput,
    InputFlush,

    EndPreUpdate,

    UpdateJoin,
    Update,

    StartPostUpdate,

    UpdateKeymap,

    ProcessWindowAction,
    Clean,
    CleanFlush,
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
            PreUpdate,
            (
                DWayStartSet::CreateDisplay,
                DWayStartSet::CreateDisplayFlush,
                DWayStartSet::Spawn,
            )
                .before(EndPreUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (
                UpdateGeometry,
                UpdateXWayland.after(UpdateGeometry),
                InitDmaBufFeedback,
                UpdateJoin.after(UpdateGeometry),
                UpdateAppInfo,
                UpdateSurface.after(UpdateGeometry),
            )
                .before(EndPreUpdate)
                .after(Dispatch)
                .before(Update),
        );
        app.configure_sets(
            PreUpdate,
            (Create, CreateGlobal, Dispatch, UpdateJoin, Update)
                .chain()
                .before(EndPreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (Input, GrabInput, InputFlush)
                .chain()
                .before(Create)
                .before(EndPreUpdate),
        );

        app.configure_sets(
            bevy::prelude::PostUpdate,
            (UpdateKeymap,).after(StartPostUpdate),
        );

        app.configure_sets(
            bevy::prelude::Last,
            (ProcessWindowAction, Clean, CleanFlush)
                .chain()
                .ambiguous_with_all(),
        );

        app.add_systems(
            Startup,
            apply_deferred.in_set(DWayStartSet::CreateDisplayFlush),
        );
        app.add_systems(bevy::prelude::PreUpdate, apply_deferred.in_set(InputFlush));
        app.add_systems(bevy::prelude::Last, apply_deferred.in_set(CleanFlush));
    }
}
