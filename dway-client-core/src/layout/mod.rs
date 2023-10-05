use crate::prelude::*;

#[derive(Component)]
pub struct Slot;

bitflags::bitflags! {
    #[derive(Clone,Copy, Debug,Hash,PartialEq, Eq, PartialOrd, Ord)]
    pub struct LayoutFlags: u64 {
        const FULL_SCREEN = 1;
        const BACKGROUND = 2;
        const FLOAT = 4;
        const ALL_SCREEN = 8;
        const ALL_WORKSPACE = 10;
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct LayoutRect {
    pub top: i32,
    pub buttom: i32,
    pub left: i32,
    pub eight: i32,
}

#[derive(Component, PartialEq, Eq, Hash, Debug, Clone)]
pub struct LayoutStyle {
    pub flag: LayoutFlags,
    pub pedding: LayoutRect,
    pub margin: LayoutRect,
    pub min_size:IVec2,
    pub max_size:IVec2,
}

relationship!(WindowInSlot=>SlotRef>-WinodwList);

pub struct LayoutPlugin;
impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowInSlot>();
    }
}
