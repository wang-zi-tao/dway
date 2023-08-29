use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::Component,
};
use bitflags::bitflags;

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    schedule::DWayServerSet,
    util::rect::IRect,
    wl::surface::WlSurface,
    x11::window::{XWindow, XWindowRef},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel},
};

use super::{
    keyboard::WlKeyboard,
    pointer::WlPointer,
    seat::{PointerList, WlSeat},
};

#[derive(Debug)]
pub enum GrabEventKind {
    PointerMove(Vec2),
    PointerButton(MouseButtonInput, DVec2),
    PointerAxis(MouseWheel, DVec2),
    Keyboard(KeyboardInput),
}
pub struct GrabEvent {
    pub seat_entity: Entity,
    pub event_kind: GrabEventKind,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash, Reflect, FromReflect)]
    pub struct ResizeEdges: u8 {
        const TOP =     0b00000001;
        const BUTTOM =  0b00000010;
        const LEFT =    0b00000100;
        const RIGHT =   0b00001000;
    }
}

#[derive(Component, Debug, Default, Reflect, FromReflect)]
#[reflect(Debug)]
pub enum Grab {
    #[default]
    None,
    OnPopup {
        surface_entity: Entity,
        #[reflect(ignore)]
        popup_stack: Vec<xdg_popup::XdgPopup>,
        pressed: bool,
        serial: u32,
    },
    ButtonDown {
        surface: Entity,
    },
    Moving {
        surface: Entity,
        serial: u32,
        relative: IVec2,
    },
    Resizing {
        surface: Entity,
        #[reflect(ignore)]
        edges: ResizeEdges,
        serial: u32,
        relative: IVec2,
        origin_rect: IRect,
    },
}

impl Drop for Grab {
    fn drop(&mut self) {
        match self {
            Grab::OnPopup { popup_stack, .. } => popup_stack.iter().rev().for_each(|popup| {
                if popup.is_alive() {
                    popup.popup_done();
                }
            }),
            _ => {}
        }
    }
}
pub fn on_grab_event(
    mut seat_query: Query<(&PointerList, &mut WlSeat, &mut Grab)>,
    mut surface_query: Query<(
        &WlSurface,
        &mut Geometry,
        &GlobalGeometry,
        Option<&XdgPopup>,
        Option<&XdgToplevel>,
        Option<&XWindowRef>,
    )>,
    mut xwindow_query: Query<&mut XWindow>,
    mut pointer_query: Query<&mut WlPointer>,
    mut keyboard_query: Query<&mut WlKeyboard>,
    mut event: EventReader<GrabEvent>,
    _commands: Commands,
) {
    for GrabEvent {
        seat_entity,
        event_kind,
    } in event.iter()
    {
        if let Ok((pointer_list, mut seat, mut grab)) = seat_query.get_mut(*seat_entity) {
            match &mut *grab {
                Grab::OnPopup {
                    surface_entity,
                    popup_stack: _,
                    pressed,
                    serial: _,
                } => {
                    let Ok((surface, _geo, global_geo, ..)) =
                        surface_query.get_mut(*surface_entity)
                    else {
                        return;
                    };
                    match event_kind {
                        GrabEventKind::PointerMove(pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    pointer.move_cursor(&mut seat, surface, relative.as_vec2());
                                }
                            }
                        }
                        GrabEventKind::PointerButton(event, pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    pointer.button(&mut seat, event, surface, relative.as_dvec2());
                                }
                            }
                            match (event.state, *pressed) {
                                (ButtonState::Pressed, false) => {
                                    *pressed = true;
                                    seat.grab(surface);
                                }
                                (ButtonState::Released, true) => {
                                    *pressed = false;
                                    seat.unset_grab();
                                }
                                _ => {}
                            }
                        }
                        GrabEventKind::PointerAxis(event, pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    let acc = |x: f64| x * 20.0;
                                    pointer.asix(
                                        &mut seat,
                                        DVec2::new(-acc(event.x as f64), -acc(event.y as f64)),
                                        surface,
                                        relative.as_dvec2(),
                                    );
                                }
                            }
                        }
                        GrabEventKind::Keyboard(input) => {
                            for e in pointer_list.iter() {
                                if let Ok(mut keyboard) = keyboard_query.get_mut(e) {
                                    keyboard.key(surface, input);
                                }
                            }
                        }
                    }
                }
                Grab::ButtonDown {
                    surface: surface_entity,
                } => {
                    let Ok((surface, _geo, global_geo, ..)) =
                        surface_query.get_mut(*surface_entity)
                    else {
                        return;
                    };
                    match event_kind {
                        GrabEventKind::PointerMove(pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    pointer.move_cursor(&mut seat, surface, relative.as_vec2());
                                }
                            }
                        }
                        GrabEventKind::PointerButton(event, pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    pointer.button(&mut seat, event, surface, relative.as_dvec2());
                                }
                            }
                            match event.state {
                                ButtonState::Pressed => {
                                    seat.grab(surface);
                                }
                                ButtonState::Released => {
                                    *grab = Grab::None;
                                    seat.unset_grab();
                                }
                            }
                        }
                        GrabEventKind::PointerAxis(event, pos) => {
                            let relative = pos.as_ivec2()
                                - global_geo.geometry.pos()
                                - surface.image_rect().pos();
                            for e in pointer_list.iter() {
                                if let Ok(mut pointer) = pointer_query.get_mut(e) {
                                    let acc = |x: f64| x * 20.0;
                                    pointer.asix(
                                        &mut seat,
                                        DVec2::new(-acc(event.x as f64), -acc(event.y as f64)),
                                        surface,
                                        relative.as_dvec2(),
                                    );
                                }
                            }
                        }
                        GrabEventKind::Keyboard(input) => {
                            for e in pointer_list.iter() {
                                if let Ok(mut keyboard) = keyboard_query.get_mut(e) {
                                    keyboard.key(surface, input);
                                }
                            }
                        }
                    }
                }
                Grab::Moving {
                    surface,
                    serial: _,
                    relative,
                } => {
                    let Ok((_surface, mut geo, _global_geo, _, toplevel, xwindow_ref)) =
                        surface_query.get_mut(*surface)
                    else {
                        return;
                    };
                    match event_kind {
                        GrabEventKind::PointerMove(_pos) => {
                            let pos = seat.pointer_position.unwrap_or_default();
                            geo.set_pos(*relative + pos);
                            if let Some(mut xwindow) = xwindow_ref
                                .and_then(|r| r.get())
                                .and_then(|e| xwindow_query.get_mut(e).ok())
                            {
                                if let Err(e) = xwindow.set_rect(geo.geometry) {
                                    error!("failed to resize window: {e}");
                                }
                            }
                        }
                        GrabEventKind::PointerAxis(_, _) | GrabEventKind::Keyboard(_) => {
                            *grab = Grab::None;
                            seat.enable();
                            info!("stop moving");
                        }
                        GrabEventKind::PointerButton(e, _) => {
                            if e.state == ButtonState::Released {
                                *grab = Grab::None;
                                seat.enable();
                                info!("stop moving");
                            }
                        }
                    }
                }
                Grab::Resizing {
                    surface,
                    edges,
                    serial: _,
                    relative,
                    origin_rect,
                } => {
                    let Ok((_surface, mut geo, _global_geo, _, toplevel, xwindow_ref)) =
                        surface_query.get_mut(*surface)
                    else {
                        return;
                    };
                    match event_kind {
                        GrabEventKind::PointerMove(_pos) => {
                            let pos = seat.pointer_position.unwrap_or_default();
                            let top_left = *relative + pos;
                            let buttom_right = top_left + origin_rect.size();
                            if edges.contains(ResizeEdges::LEFT) {
                                geo.min.x = top_left.x;
                            }
                            if edges.contains(ResizeEdges::TOP) {
                                geo.min.y = top_left.y;
                            }
                            if edges.contains(ResizeEdges::RIGHT) {
                                geo.max.x = buttom_right.x;
                            }
                            if edges.contains(ResizeEdges::BUTTOM) {
                                geo.max.y = buttom_right.y;
                            }
                            toplevel.map(|t| t.resize(geo.size()));
                            if let Some(mut xwindow) = xwindow_ref
                                .and_then(|r| r.get())
                                .and_then(|e| xwindow_query.get_mut(e).ok())
                            {
                                if let Err(e) = xwindow.resize(geo.geometry) {
                                    error!("failed to move window: {e}");
                                }
                            }
                        }
                        GrabEventKind::PointerAxis(_, _)
                        | GrabEventKind::Keyboard(_)
                        | GrabEventKind::PointerButton(..) => {
                            *grab = Grab::None;
                            seat.enable();
                            info!("stop resizing");
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct GrabPlugin;
impl Plugin for GrabPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Grab>();
        app.add_event::<GrabEvent>();
        app.add_system(
            on_grab_event
                .run_if(on_event::<GrabEvent>())
                .in_set(DWayServerSet::GrabInput),
        );
    }
}
