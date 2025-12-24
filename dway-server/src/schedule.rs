use bevy::{ecs::schedule::ScheduleLabel, ui::UiSystem};
use dway_util::eventloop::PollerSystems;

use crate::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DWayStartSet {
    CreateDisplay,
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
    UpdateImage,
    UpdateClipboard,

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
            Startup,
            (
                DWayStartSet::CreateDisplay,
                DWayStartSet::Spawn,
            )
                .before(EndPreUpdate)
                .chain()
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (
                UpdateImage,
                UpdateGeometry.after(UpdateImage),
                UpdateXWayland.after(UpdateGeometry),
                InitDmaBufFeedback,
                UpdateJoin.after(UpdateGeometry),
                UpdateAppInfo,
                UpdateSurface.after(UpdateGeometry),
                UpdateClipboard,
            )
                .before(EndPreUpdate)
                .after(Dispatch)
                .before(Update)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (Create, CreateGlobal)
                .chain()
                .before(EndPreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (Dispatch, UpdateJoin, Update)
                .chain()
                .before(EndPreUpdate)
                .ambiguous_with_all(),
        );
        app.configure_sets(
            PreUpdate,
            (
                Input.before(UpdateGeometry).after(UiSystem::Focus),
                GrabInput,
                InputFlush,
            )
                .chain()
                .before(EndPreUpdate)
                .ambiguous_with_all(),
        );

        app.configure_sets(
            bevy::prelude::PostUpdate,
            (UpdateKeymap,).after(StartPostUpdate).ambiguous_with_all(),
        );

        app.configure_sets(
            bevy::prelude::Last,
            (
                ProcessWindowAction,
                Clean,
                CleanFlush.before(PollerSystems::Flush),
            )
                .chain()
                .ambiguous_with_all(),
        );
    }
}
