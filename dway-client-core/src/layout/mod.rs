pub mod equalsize;
pub mod lsp;
pub mod tile;
use dway_server::{geometry::Geometry, util::rect::IRect, xdg::DWayWindow};

use crate::{prelude::*, workspace::Workspace, DWayClientSystem};

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
    pub min_size: IVec2,
    pub max_size: IVec2,
}

#[derive(Component, Reflect, PartialEq, Eq, Hash, Debug, Clone)]
pub struct CalculatedWindowGeometry {
    pub geometry: IRect,
}

relationship!(WindowInSlot=>SlotRef>-WinodwList);
relationship!(WorkspaceHasSlot=>SlotList-<WorkspaceList);
pub struct RefreshLayout(pub Entity);

pub fn attach_window_to_slot(
    workspace_query: Query<
        (&SlotList, &super::workspace::WindowList),
        Or<(Changed<SlotList>, Changed<super::workspace::WindowList>)>,
    >,
    slot_query: Query<(&Geometry), With<Slot>>,
    mut commands: Commands,
) {
    workspace_query.for_each(|(slots, windows)| {
        windows
            .iter()
            .zip(slots.iter().cycle().take(windows.len()))
            .for_each(|(window, slot)| {
                commands.add(ConnectCommand::<WindowInSlot>::new(window, slot));
                if let Ok(geo) = slot_query.get(slot) {
                    commands.entity(slot).insert(CalculatedWindowGeometry {
                        geometry: geo.geometry,
                    });
                }
            });
    });
}

pub struct LayoutPlugin;
impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowInSlot>();
        app.register_relation::<WorkspaceHasSlot>();
        app.register_type::<CalculatedWindowGeometry>();
        app.add_system(attach_window_to_slot.in_set(DWayClientSystem::UpdateWindowGeometry));
        app.add_plugin(tile::TileLayoutPlugin);
    }
}
