pub mod equalsize;
pub mod lsp;
pub mod tile;

use crate::{prelude::*, DWayClientSystem};
use dway_server::{
    geometry::Geometry,
    util::rect::IRect,
    xdg::{
        toplevel::{DWayToplevel, PinedWindow},
        DWayWindow,
    },
};

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
    pub padding: LayoutRect,
    pub margin: LayoutRect,
    pub min_size: IVec2,
    pub max_size: IVec2,
}
impl LayoutStyle {
    pub fn get_pedding_rect(&self, rect: IRect) -> IRect {
        IRect {
            min: IVec2 {
                x: rect.x() + self.padding.left,
                y: rect.y() + self.padding.top,
            },
            max: IVec2 {
                x: rect.max.x - self.padding.right,
                y: rect.max.y - self.padding.buttom,
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
    mut window_query: Query<&mut Geometry, (With<DWayWindow>, With<DWayToplevel>, Without<Slot>)>,
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
                        commands.entity(window).insert(PinedWindow);
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
            PreUpdate,
            (attach_window_to_slot, apply_deferred)
                .chain()
                .in_set(DWayClientSystem::UpdateWindowGeometry),
        );
        app.add_plugins(tile::TileLayoutPlugin);
    }
}
