pub mod equalsize;
pub mod lsp;
pub mod tile;
use dway_server::{
    geometry::Geometry,
    util::rect::IRect,
    xdg::{toplevel::XdgToplevel, DWayToplevelWindow, DWayWindow, XdgSurface},
};

use crate::{prelude::*, workspace::Workspace, DWayClientSystem};

#[derive(Component)]
pub struct Slot;

bitflags::bitflags! {
    #[derive(Clone,Copy,Default, Debug,Hash,PartialEq, Eq, PartialOrd, Ord)]
    pub struct LayoutFlags: u64 {
        const FULL_SCREEN = 1;
        const BACKGROUND = 2;
        const FLOAT = 4;
        const ALL_SCREEN = 8;
        const ALL_WORKSPACE = 10;
    }
}

#[derive(PartialEq, Default, Eq, Hash, Debug, Clone, Copy)]
pub struct LayoutRect {
    pub top: i32,
    pub buttom: i32,
    pub left: i32,
    pub right: i32,
}

impl LayoutRect {
    pub fn new(pixel: i32) -> Self {
        Self {
            top: pixel,
            buttom: pixel,
            left: pixel,
            right: pixel,
        }
    }
}

#[derive(Component, Default, PartialEq, Eq, Hash, Debug, Clone)]
pub struct LayoutStyle {
    pub flag: LayoutFlags,
    pub pedding: LayoutRect,
    pub margin: LayoutRect,
    pub min_size: IVec2,
    pub max_size: IVec2,
}
impl LayoutStyle {
    pub fn get_pedding_rect(&self, rect: IRect) -> IRect {
        IRect {
            min: IVec2 {
                x: rect.x() + self.pedding.left,
                y: rect.y() + self.pedding.top,
            },
            max: IVec2 {
                x: rect.max.x - self.pedding.right,
                y: rect.max.y - self.pedding.buttom,
            },
        }
    }
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
        (
            &SlotList,
            &super::workspace::WindowList,
            Option<&LayoutStyle>,
        ),
        Or<(
            Changed<SlotList>,
            Changed<super::workspace::WindowList>,
            Changed<LayoutStyle>,
        )>,
    >,
    slot_query: Query<&Geometry, With<Slot>>,
    mut window_query: Query<
        &mut Geometry,
        (With<DWayWindow>, With<DWayToplevelWindow>, Without<Slot>),
    >,
    mut commands: Commands,
    mut window_actions: EventWriter<WindowAction>,
) {
    workspace_query.for_each(|(slots, windows, layout_style)| {
        windows
            .iter()
            .zip(slots.iter().cycle().take(windows.len()))
            .for_each(|(window, slot)| {
                commands.add(ConnectCommand::<WindowInSlot>::new(window, slot));
                if let Ok(geo) = slot_query.get(slot) {
                    let rect = layout_style
                        .map(|s| s.get_pedding_rect(geo.geometry))
                        .unwrap_or(geo.geometry);
                    commands
                        .entity(slot)
                        .insert(CalculatedWindowGeometry { geometry: rect });
                    if let Ok(mut window_geo) = window_query.get_mut(window) {
                        window_geo.geometry = rect;
                        window_actions.send(WindowAction::SetRect(window, rect));
                    }
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
        app.add_systems(
            (attach_window_to_slot, apply_system_buffers)
                .chain()
                .in_set(DWayClientSystem::UpdateWindowGeometry),
        );
        app.add_plugin(tile::TileLayoutPlugin);
    }
}
