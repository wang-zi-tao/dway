pub mod equalsize;
pub mod lsp;
pub mod tile;

use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::{
        toplevel::{DWayToplevel, PinedWindow},
        DWayWindow,
    },
};
use dway_util::update;

use self::tile::{TileLayoutKind, WindowWithoutTile};
use crate::{prelude::*, screen::Screen, workspace::ScreenAttachWorkspace};

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

#[derive(PartialEq, Reflect, Default, Eq, Hash, Debug, Clone, Copy)]
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

    pub fn inner_rect(&self, rect: IRect) -> IRect {
        IRect {
            min: IVec2 {
                x: rect.x() + self.left,
                y: rect.y() + self.top,
            },
            max: IVec2 {
                x: rect.max.x - self.right,
                y: rect.max.y - self.buttom,
            },
        }
    }
}

#[derive(Component, Reflect, Default, PartialEq, Eq, Hash, Debug, Clone)]
pub struct LayoutStyle {
    #[reflect(ignore)]
    pub flag: LayoutFlags,
    pub padding: LayoutRect,
    pub margin: LayoutRect,
    pub geometry: Option<IRect>,
    pub min_size: IVec2,
    pub max_size: IVec2,
}
impl LayoutStyle {
    pub fn get_pedding_rect(&self, rect: IRect) -> IRect {
        self.padding.inner_rect(rect)
    }
}

pub fn calculate_geometry(
    parent_geo: IRect,
    parent_layout: Option<&LayoutStyle>,
    layout: Option<&LayoutStyle>,
) -> IRect {
    let padding = parent_layout.map(|l| l.padding).unwrap_or_default();
    let parent_inner = padding.inner_rect(parent_geo);
    let geometry = layout.and_then(|l| l.geometry);
    let outter_rect = IRect::from_pos_size(
        parent_inner.pos() + geometry.map(|g| g.pos()).unwrap_or_default(),
        geometry.map(|g| g.size()).unwrap_or(parent_inner.size()),
    );
    let margins = layout.map(|l| l.margin).unwrap_or_default();
    margins.inner_rect(outter_rect)
}

graph_query2! {GeometryGraph=>
screen_workspace_path=match (screen: (Entity) filter With<Screen>)-[ScreenAttachWorkspace]->(workspace: Entity);
slot_window_path=match (screen: (Entity) filter With<Slot>)<-[WindowInSlot]-(window: Entity filter With<DWayWindow>);
}

pub fn update_geometry(
    graph: GeometryGraph,
    mut geometry_query: Query<(&mut Geometry, Option<&LayoutStyle>)>,
    mut global_geometry_query: Query<(&mut GlobalGeometry, Option<&LayoutStyle>)>,
) {
    let mut do_update = |p, c| {
        if let ((Ok((parent_geo, parent_layout)), Ok((mut geo, layout)))) =
            (global_geometry_query.get(p), geometry_query.get_mut(c))
        {
            let calculated_geo = calculate_geometry(parent_geo.geometry, parent_layout, layout);
            update!(geo.geometry, calculated_geo);
        }
    };
    graph.foreach_screen_workspace_path(|&screen, &workspace| {
        do_update(screen, workspace);
        ControlFlow::<()>::Continue
    });
    graph.foreach_slot_window_path(|&slot, &window| {
        do_update(slot, window);
        ControlFlow::<()>::Continue
    });
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
            Ref<TileLayoutKind>,
        ),
        Or<(
            Changed<SlotList>,
            Changed<super::workspace::WindowList>,
            Changed<LayoutStyle>,
            Changed<TileLayoutKind>,
        )>,
    >,
    slot_query: Query<&Geometry, With<Slot>>,
    mut window_query: Query<
        &mut Geometry,
        (
            With<DWayWindow>,
            With<DWayToplevel>,
            Without<Slot>,
            Without<WindowWithoutTile>,
        ),
    >,
    mut commands: Commands,
    mut window_actions: EventWriter<WindowAction>,
) {
    for (slots, windows, layout_style, tile) in workspace_query.iter() {
        windows
            .iter()
            .zip(slots.iter().cycle().take(windows.len()))
            .for_each(|(window, slot)| {
                commands.queue(ConnectCommand::<WindowInSlot>::new(window, slot));
                commands.entity(window).insert(PinedWindow);
            });
        if tile.is_changed() {
            match &*tile {
                TileLayoutKind::Float => {
                    for window in windows.iter() {
                        commands.entity(window).remove::<PinedWindow>();
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct LayoutPlugin;
impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowInSlot>();
        app.register_relation::<WorkspaceHasSlot>();
        app.register_type::<CalculatedWindowGeometry>();
        app.register_type::<LayoutStyle>();
        app.add_systems(
            PreUpdate,
            (
                attach_window_to_slot.in_set(DWayClientSystem::UpdateWindowGeometry),
                update_geometry.in_set(DWayClientSystem::UpdateLayout),
            ),
        );
        app.add_plugins(tile::TileLayoutPlugin);
    }
}
