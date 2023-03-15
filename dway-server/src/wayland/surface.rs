use std::{
    cell::{RefCell, RefMut},
    sync::{Mutex, MutexGuard},
};
use smithay::{
    desktop::{
        space::SpaceElement, utils::with_surfaces_surface_tree, PopupKind, Window,
    },
    reexports::wayland_server::{
        backend::smallvec::SmallVec, protocol::wl_surface::WlSurface, Resource,
    },
    utils::{Logical, Physical, Rectangle, Scale},
    wayland::{
        compositor::with_states,
        seat::WaylandFocus,
        shell::xdg::{XdgPopupSurfaceRoleAttributes, XdgToplevelSurfaceRoleAttributes},
    },
    xwayland::X11Surface,
};
use uuid::Uuid;

use super::{
    shell::{ResizeState, WindowElement},
    DWayState,
};

pub struct DWaySurfaceData {
    pub uuid: Uuid,
    pub geo: Rectangle<i32, Logical>,
    pub bbox: Rectangle<i32, Logical>,
    pub scala: Scale<i32>,
    pub resize_state: ResizeState,
    pub need_rerender: bool,
    pub ssd: bool,
}
impl DWaySurfaceData {
    pub fn new(uuid: Uuid) -> DWaySurfaceData {
        Self {
            uuid,
            scala: Scale { x: 1, y: 1 },
            resize_state: Default::default(),
            ssd: false,
            geo: Default::default(),
            bbox: Default::default(),
            need_rerender: true,
        }
    }
    pub fn with<R, F: FnOnce(&mut Self) -> R>(surface: &WlSurface, f: F) -> R {
        with_states_locked(surface, f)
    }
    pub fn try_with<R, F: FnOnce(&mut Self) -> R>(surface: &WlSurface, f: F) -> Option<R> {
        try_with_states_locked(surface, f)
    }
    pub fn with_tree<R, F: FnMut(&mut Self, &WlSurface) -> R>(
        surface: &WlSurface,
        mut f: F,
    ) -> SmallVec<[R; 1]> {
        let mut results = SmallVec::new();
        with_surfaces_surface_tree(surface, |surface, states| {
            if let Some(mut c) = try_get_component_locked::<Self>(states) {
                results.push(f(&mut c, surface));
            }
        });
        results
    }
    pub fn with_x11_surface<R, F: FnOnce(&mut Self) -> R>(window: &X11Surface, f: F) -> Option<R> {
        window
            .wl_surface()
            .and_then(|surface| try_with_states_locked(&surface, f))
    }
    pub fn with_wl_window<R, F: FnOnce(&mut Self) -> R>(window: &Window, f: F) -> Option<R> {
        window
            .wl_surface()
            .and_then(|surface| try_with_states_locked(&surface, f))
    }
    pub fn with_element<R, F: FnOnce(&mut Self) -> R>(window: &WindowElement, f: F) -> Option<R> {
        window
            .wl_surface()
            .and_then(|surface| try_with_states_locked(&surface, f))
    }
    pub fn update_x11_surface_geometry(window: &X11Surface) {
        let geo = window.geometry();
        Self::with_x11_surface(window, |s| {
            s.need_rerender=false;
            s.geo = geo;
            s.bbox = geo;
        });
    }
    pub fn get_logical_geometry_bbox(
        element: &WindowElement,
    ) -> Option<(Rectangle<i32, Logical>, Rectangle<i32, Logical>)> {
        let element_geo = element.geometry();
        let element_bbox = element.bbox();
        element.wl_surface().and_then(|surface| {
            Self::try_with(&surface, |s| {
                let outer_geo = s.geo;
                let _outer_bbox = s.bbox;
                let mut bbox = element_bbox;
                let mut geo = element_geo;
                bbox.loc += outer_geo.loc - geo.loc;
                geo.loc = outer_geo.loc;
                (geo, bbox)
            })
        })
    }
    pub fn get_physical_geometry_bbox(
        element: &WindowElement,
    ) -> Option<(Rectangle<i32, Physical>, Rectangle<i32, Physical>)> {
        let element_geo = element.geometry();
        let element_bbox = element.bbox();
        element.wl_surface().and_then(|surface| {
            Self::try_with(&surface, |s| {
                let outer_geo = s.geo.to_physical(s.scala);
                let _outer_bbox = s.bbox.to_physical(s.scala);
                let mut bbox = element_bbox.to_physical(s.scala);
                let mut geo = element_geo.to_physical(s.scala);
                bbox.loc += outer_geo.loc - geo.loc;
                geo.loc = outer_geo.loc;
                (geo, bbox)
            })
        })
    }
}

pub fn try_with_states_locked<F, T, C>(surface: &WlSurface, f: F) -> Option<T>
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        states
            .data_map
            .get::<Mutex<C>>()
            .and_then(|l| l.lock().ok())
            .map(|mut l| f(&mut l))
    })
}
pub fn with_states_locked<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = get_component_locked(states);
        f(&mut state)
    })
}
pub fn get_component_locked<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> MutexGuard<C> {
    states.data_map.get::<Mutex<C>>().unwrap().lock().unwrap()
}
pub fn try_get_component_locked<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> Option<MutexGuard<C>> {
    states
        .data_map
        .get::<Mutex<C>>()
        .and_then(|l| l.lock().ok())
}
pub fn try_get_component_borrowed<C: 'static>(
    states: &smithay::wayland::compositor::SurfaceData,
) -> Option<RefMut<C>> {
    states
        .data_map
        .get::<RefCell<C>>()
        .map(|l| l.borrow_mut())
}

pub fn ensure_initial_configure(dway: &mut DWayState, surface: &WlSurface) {

    if let Some(window) = dway
        .element_map
        .values()
        .find(|window| window.wl_surface().map(|s| s == *surface).unwrap_or(false))
        .cloned()
    {
        // send the initial configure if relevant
        if let WindowElement::Wayland(ref toplevel) = window {
            let initial_configure_sent =
                with_states_locked(surface, |s: &mut XdgToplevelSurfaceRoleAttributes| {
                    s.initial_configure_sent
                });
            if !initial_configure_sent {
                toplevel.toplevel().send_configure();
            }
        }

        with_states_locked(surface, |data: &mut DWaySurfaceData| {
            if let ResizeState::WaitingForCommit(_) = data.resize_state {
                data.resize_state = ResizeState::NotResizing;
            }
        });
        return;
    }

    if let Some(popup) = dway.popups.find_popup(surface) {
        let PopupKind::Xdg(ref popup) = popup;
        let initial_configure_sent =
            with_states_locked(surface, |s: &mut XdgPopupSurfaceRoleAttributes| {
                s.initial_configure_sent
            });
        if !initial_configure_sent {
            // NOTE: This should never fail as the initial configure is always
            // allowed.
            popup.send_configure().expect("initial configure failed");
        }
    }

    // if let Some(output) = dway.space.outputs().find(|o| {
    //     let map = layer_map_for_output(o);
    //     map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
    //         .is_some()
    // }) {
    //     let initial_configure_sent =
    //         with_states_locked(surface, |s: &mut LayerSurfaceAttributes| {
    //             s.initial_configure_sent
    //         });
    //
    //     // send the initial configure if relevant
    //     if !initial_configure_sent {
    //         let mut map = layer_map_for_output(output);
    //
    //         // arrange the layers before sending the initial configure
    //         // to respect any size the client may have sent
    //         map.arrange();
    //
    //         let layer = map
    //             .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
    //             .unwrap();
    //
    //         layer.layer_surface().send_configure();
    //     }
    // };
}

pub fn print_surface_tree(surface: &WlSurface) {
    print!("root: {} tree: [", surface.id());
    with_surfaces_surface_tree(surface, |surface, _states| {
        print!("  {}", surface.id());
    });
    println!("]");
}
