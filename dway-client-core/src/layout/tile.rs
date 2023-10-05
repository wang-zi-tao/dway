use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
};

use crate::{
    layout::WorkspaceHasSlot,
    prelude::*,
    workspace::{self, Workspace},
    DWayClientSystem,
};

use super::{Slot, SlotList};

#[derive(Component, Clone, Debug, Reflect)]
pub enum TileLayoutKind {
    Horizontal,
    Vertical,
    Grid,
    TileLeft { split: f32 },
    Fullscreen,
}

impl TileLayoutKind {
    pub fn apply(&self, window_count: usize) -> Vec<Rect> {
        match self {
            TileLayoutKind::Horizontal => (0..window_count)
                .map(|i| {
                    Rect::new(
                        i as f32 / window_count as f32,
                        0.0,
                        (i + 1) as f32 / window_count as f32,
                        1.0,
                    )
                })
                .collect(),
            TileLayoutKind::Vertical => (0..window_count)
                .map(|i| {
                    Rect::new(
                        0.0,
                        i as f32 / window_count as f32,
                        1.0,
                        (i + 1) as f32 / window_count as f32,
                    )
                })
                .collect(),
            TileLayoutKind::Grid => {
                let col_count = (window_count as f32).sqrt().floor() as usize;
                if col_count * col_count == window_count {
                    (0..col_count)
                        .flat_map(|i| {
                            (0..col_count).map(move |j| {
                                Rect::new(
                                    i as f32 / window_count as f32,
                                    j as f32 / window_count as f32,
                                    (i + 1) as f32 / window_count as f32,
                                    (j + 1) as f32 / window_count as f32,
                                )
                            })
                        })
                        .collect()
                } else {
                    let mut rects = vec![];
                    let area = 1.0 / window_count as f32;
                    if col_count > 0 {
                        for i in 0..col_count {
                            for j in 0..col_count - 1 {
                                rects.push(Rect::new(
                                    i as f32 / window_count as f32,
                                    (area * col_count as f32) * j as f32,
                                    (i + 1) as f32 / window_count as f32,
                                    (area * col_count as f32) * (j + 1) as f32,
                                ));
                            }
                        }
                    }
                    let last_row_len = window_count - col_count * col_count;
                    for i in 0..last_row_len {
                        rects.push(Rect::new(
                            area * (col_count as f32) * (col_count - 1) as f32,
                            i as f32 / window_count as f32,
                            area * (col_count as f32) * (col_count - 1) as f32,
                            (i + 1) as f32 / window_count as f32,
                        ));
                    }
                    rects
                }
            }
            TileLayoutKind::TileLeft { split } => {
                let mut rects = vec![];
                if window_count == 1 {
                    rects.push(Rect::new(0.0, 0.0, 1.0, 1.0));
                } else if window_count > 1 {
                    rects.push(Rect::new(0.0, 0.0, *split, 1.0));
                }
                for i in 0..window_count - 1 {
                    rects.push(Rect::new(
                        *split,
                        (i / (window_count - 1)) as f32,
                        1.0,
                        ((i + 1) / (window_count - 1)) as f32,
                    ));
                }
                rects
            }
            TileLayoutKind::Fullscreen => (0..window_count)
                .map(|_| Rect::new(0.0, 0.0, 1.0, 1.0))
                .collect(),
        }
    }
}

pub fn update_tile_layout(
    workspace: Query<
        (
            Entity,
            &Geometry,
            &GlobalGeometry,
            &workspace::WindowList,
            &TileLayoutKind,
        ),
        Or<(Changed<workspace::WindowList>, Changed<TileLayoutKind>)>,
    >,
    mut commands: Commands,
) {
    workspace.for_each(|(entity, geometry, global_geometry, windows, layout)| {
        debug!(workspace=?entity,window_count=%windows.len(), "refresh tile layout");
        commands.add(DespawnAllConnectedEntityCommand::<WorkspaceHasSlot>::new(
            entity,
        ));
        let slots = layout.apply(windows.len());
        for rect in slots {
            let slot_rect = IRect::new(
                (rect.min.x as f32 * geometry.width() as f32) as i32,
                (rect.min.y as f32 * geometry.height() as f32) as i32,
                (rect.width() as f32 * geometry.width() as f32) as i32,
                (rect.height() as f32 * geometry.height() as f32) as i32,
            );
            let slot_geo = Geometry::new(slot_rect);
            let slot_entity = commands
                .spawn((Slot, global_geometry.add(&slot_geo), slot_geo))
                .set_parent(entity)
                .id();
            commands.add(ConnectCommand::<WorkspaceHasSlot>::new(entity, slot_entity));
        }
    });
}

pub struct TileLayoutPlugin;
impl Plugin for TileLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TileLayoutKind>();
        app.add_system(
            update_tile_layout
                .before(super::attach_window_to_slot)
                .in_set(DWayClientSystem::UpdateLayout),
        );
    }
}
