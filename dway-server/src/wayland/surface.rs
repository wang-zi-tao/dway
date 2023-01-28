use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    sync::{
        Mutex,
    },
};

use bevy_input::{keyboard::KeyboardInput, mouse::MouseButtonInput, prelude::MouseButton};
use bevy_math::Vec2;
use crossbeam_channel::{Receiver, Sender};
use failure::{format_err, Fallible};
use slog::{debug, error, info, trace, warn};
use smithay::{
    desktop::{
        layer_map_for_output, PopupKind, PopupManager, Space, WindowSurfaceType,
    },
    reexports::wayland_server::{
            protocol::{wl_surface::WlSurface},
        },
    utils::{
        Logical, Rectangle,
    },
    wayland::{
        compositor::{
            with_states, with_surface_tree_upward, TraversalAction,
        },
        seat::WaylandFocus,
        shell::{
            wlr_layer::{
                LayerSurfaceAttributes,
            },
            xdg::{
                XdgPopupSurfaceRoleAttributes,
                XdgToplevelSurfaceRoleAttributes,
            },
        },
    },
};
use uuid::Uuid;


use super::{
    shell::{ResizeState, WindowElement},
};


#[derive(Default)]
pub struct SurfaceData {
    pub uuid:Uuid,
    pub geometry: Option<Rectangle<i32, Logical>>,
    pub resize_state: ResizeState,
}

pub fn with_states_locked<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = states.data_map.get::<Mutex<C>>().unwrap().lock().unwrap();
        f(&mut state)
    })
}
pub fn with_states_borrowed<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = states.data_map.get::<RefCell<C>>().unwrap().borrow();
        f(&mut state)
    })
}
pub fn with_states_borrowed_mut<F, T, C>(surface: &WlSurface, f: F) -> T
where
    F: FnOnce(&mut C) -> T,
    C: 'static,
{
    with_states(surface, |states| {
        let mut state = states.data_map.get::<RefCell<C>>().unwrap().borrow_mut();
        f(&mut state)
    })
}

pub fn ensure_initial_configure(
    surface: &WlSurface,
    space: &Space<WindowElement>,
    popups: &mut PopupManager,
) {
    with_surface_tree_upward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |_, states, _| {
            states
                .data_map
                .insert_if_missing(|| RefCell::new(SurfaceData::default()));
        },
        |_, _, _| true,
    );

    if let Some(window) = space
        .elements()
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

        with_states_borrowed_mut(surface, |data: &mut SurfaceData| {
            if let ResizeState::WaitingForCommit(_) = data.resize_state {
                data.resize_state = ResizeState::NotResizing;
            }
        });
        return;
    }

    if let Some(popup) = popups.find_popup(surface) {
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

        return;
    };

    if let Some(output) = space.outputs().find(|o| {
        let map = layer_map_for_output(o);
        map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
            .is_some()
    }) {
        let initial_configure_sent =
            with_states_locked(surface, |s: &mut LayerSurfaceAttributes| {
                s.initial_configure_sent
            });

        // send the initial configure if relevant
        if !initial_configure_sent {
            let mut map = layer_map_for_output(output);

            // arrange the layers before sending the initial configure
            // to respect any size the client may have sent
            map.arrange();

            let layer = map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .unwrap();

            layer.layer_surface().send_configure();
        }
    };
}

