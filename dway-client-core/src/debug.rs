use std::any::type_name;

use bevy::prelude::*;

use crate::{DWayClientState, DWayClientSystem};

#[derive(Default)]
pub struct DebugPlugin {}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(
            pring_stage_stack::<DWayClientState>
                .in_base_set(CoreSet::PostUpdate)
                .after(DWayClientSystem::UpdateState),
        );
    }
}

pub fn pring_stage_stack<S: States>(stages: Res<State<S>>) {
    if stages.is_changed() {
        let type_name = type_name::<S>();
        info!("stages {} {:?}", type_name, &*stages);
    }
}
