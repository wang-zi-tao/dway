use dway_server::{
    events::Insert,
    geometry::GlobalGeometry,
    wl::surface::WlSurface,
    xdg::{toplevel::XdgToplevel, self},
};



use bevy::{
    prelude::*,
    ui::FocusPolicy,
};


use smallvec::SmallVec;


use crate::{
    desktop::FocusedWindow,
    DWayClientSystem,
};

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Hidden;

pub struct DWayWindowPlugin;
impl Plugin for DWayWindowPlugin {
    fn build(&self, app: &mut App) {
    }
}
