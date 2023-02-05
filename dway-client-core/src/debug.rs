use std::any::type_name;

use bevy::{ecs::schedule::StateData, prelude::*, sprite::MaterialMesh2dBundle};

use crate::{stages::DWayStage, window::WindowLabel};

#[derive(Default)]
pub struct DebugPlugin {}

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(pring_stage_stack::<DWayStage>.after(WindowLabel::UpdateUi));
    }
}

pub fn pring_stage_stack<S: StateData>(stages: Res<State<S>>) {
    if stages.is_changed() && stages.inactives().len() != 0 {
        let type_name = type_name::<S>();
        info!("stages {} {:?}", type_name, &*stages);
    }
}
