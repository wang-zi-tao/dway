use std::sync::atomic::Ordering;

use bevy::prelude::*;
use smithay::{
    desktop::PopupKind,
    wayland::{compositor::get_role, shell::xdg::XDG_POPUP_ROLE},
};

use crate::{
    components::{
        GlobalPhysicalRect, LogicalRect, PhysicalRect, PopupWindow, WaylandWindow, WindowIndex,
        WindowMark, WindowScale, WlSurfaceWrapper, UUID,
    },
    events::{CommitSurface, CreatePopup, DestroyPopup, UpdatePopupPosition},
    surface::ImportedSurface,
    wayland_window::WaylandSurfaceBundle,
};

#[derive(Bundle)]
pub struct PopupBundle {
    pub surface_bundle: WaylandSurfaceBundle,
    pub popup: PopupWindow,
    pub physical_rect: PhysicalRect,
    pub global_rect: GlobalPhysicalRect,
    pub logical_rect: LogicalRect,
}

pub fn create_popup(
    mut events: EventReader<CreatePopup>,
    mut windows: ResMut<WindowIndex>,
    surfaces: Query<(Entity, Option<&WindowScale>), With<WlSurfaceWrapper>>,
    mut commands: Commands,
) {
    for e in events.iter() {
        let surface = &e.0;
        let id = surface.into();
        let uuid = UUID::new();
        let logical_rect = e.1.get_geometry();
        surface.with_pending_state(|state| {
            state.geometry = e.1.get_geometry();
        });
        if let Some((entity, scale)) = surface
            .get_parent_surface()
            .and_then(|p| windows.get(&p.into()))
            .and_then(|e| surfaces.get(*e).ok())
        {
            commands.entity(entity).with_children(|c| {
                let entity = c
                    .spawn(PopupBundle {
                        physical_rect: PhysicalRect(
                            logical_rect
                                .to_physical_precise_round(scale.cloned().unwrap_or_default().0),
                        ),
                        global_rect: GlobalPhysicalRect(
                            logical_rect
                                .to_physical_precise_round(scale.cloned().unwrap_or_default().0),
                        ),
                        logical_rect: LogicalRect(logical_rect),
                        surface_bundle: WaylandSurfaceBundle {
                            mark: WindowMark,
                            window: WlSurfaceWrapper(surface.wl_surface().clone()),
                            uuid,
                            id: surface.into(),
                        },
                        popup: PopupWindow {
                            kind: PopupKind::from(surface.clone()),
                            position: e.1.clone(),
                        },
                    })
                    .id();
                info!("create popup for {id:?} at {entity:?}");
                windows.0.insert(id, entity);
            });
        };
    }
}

pub fn reposition_request(
    mut events: EventReader<UpdatePopupPosition>,
    window_index: Res<WindowIndex>,
    mut surface_query: Query<
        (
            Entity,
            &mut PopupWindow,
            &mut LogicalRect,
            &mut PhysicalRect,
            Option<&WindowScale>,
        ),
        With<WindowMark>,
    >,
) {
    for UpdatePopupPosition {
        surface_id,
        positioner,
        token,
    } in events.iter()
    {
        if let Some((entity, mut popup, mut logical_rect, mut physical_rect, scale)) = window_index
            .get(surface_id)
            .and_then(|&e| surface_query.get_mut(e).ok())
        {
            popup.as_mut().update_with_rect(
                *positioner,
                &mut logical_rect,
                &mut physical_rect,
                scale,
            );
        }
    }
}
pub fn on_commit(
    mut events: EventReader<CommitSurface>,
    mut surface_query: Query<(
        &mut WlSurfaceWrapper,
        &mut PopupWindow,
        &mut ImportedSurface,
    )>,
    window_query: Query<&WaylandWindow>,
    window_index: Res<WindowIndex>,
) {
    for CommitSurface { surface: id, .. } in events.iter() {
        if let Some((mut wl_surface_wrapper, mut popup, mut imported_surface)) = window_index
            .get(id)
            .and_then(|&e| surface_query.get_mut(e).ok())
        {
            let surface = &mut wl_surface_wrapper;
            match &popup.kind {
                PopupKind::Xdg(w) => if get_role(surface) == Some(XDG_POPUP_ROLE) {},
            }
        }
    }
}
