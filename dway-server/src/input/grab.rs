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
    Keyboard(KeyboardInput, [u32; 4]),
}

#[derive(Event)]
pub struct GrabEvent {
    pub seat_entity: Entity,
    pub event_kind: GrabEventKind,
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
    pub struct ResizeEdges: u8 {
        const TOP =     0b00000001;
        const BUTTOM =  0b00000010;
        const LEFT =    0b00000100;
        const RIGHT =   0b00001000;
    }
}

#[derive(Component, Debug)]
pub enum SurfaceGrabKind {
    Move{
        seat: Entity,
        serial: u32,
        relative: IVec2,
    },
    Resizing{
        seat: Entity,
        edges: ResizeEdges,
        serial: u32,
        relative: IVec2,
        origin_rect: IRect,
    },
}

#[derive(Component, Debug, Default)]
pub struct WlSurfacePointerState{
    pub is_clicked:bool,
    pub mouse_pos: IVec2,
    pub grab: Option<Box<SurfaceGrabKind>>,
}
impl WlSurfacePointerState {
    pub fn is_grabed(&self) -> bool {
        self.is_clicked || self.grab.is_some()
    }
}

#[derive(Component, Debug, Default, Reflect)]
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
    mut surface_query: Query<(&WlSurface, &mut Geometry, &GlobalGeometry)>,
    mut pointer_query: Query<&mut WlPointer>,
    mut keyboard_query: Query<&mut WlKeyboard>,
    mut event: EventReader<GrabEvent>,
    mut window_action: EventWriter<WindowAction>,
    _commands: Commands,
) {
    for GrabEvent {
        seat_entity,
        event_kind,
    } in event.read()
    {
        if let Ok((pointer_list, mut seat, mut grab)) = seat_query.get_mut(*seat_entity) {
            match &mut *grab {
                Grab::OnPopup {
                    surface_entity,
                    popup_stack: _,
                    pressed,
                    serial: _,
                } => {
                    let Ok((surface, _geo, global_geo)) = surface_query.get_mut(*surface_entity)
                    else {
                        return;
                    };
                    match &event_kind {
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
                        GrabEventKind::Keyboard(input, serialize) => {
                            for e in pointer_list.iter() {
                                if let Ok(mut keyboard) = keyboard_query.get_mut(e) {
                                    keyboard.key(surface, input, *serialize);
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
                    match &event_kind {
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
                        GrabEventKind::Keyboard(input, serialize) => {
                            for e in pointer_list.iter() {
                                if let Ok(mut keyboard) = keyboard_query.get_mut(e) {
                                    keyboard.key(surface, input, *serialize);
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
                    let Ok((_surface, mut geo, _global_geo)) = surface_query.get_mut(*surface)
                    else {
                        return;
                    };
                    match event_kind {
                        GrabEventKind::PointerMove(_pos) => {
                            let pos = seat.pointer_position.unwrap_or_default();
                            geo.set_pos(*relative + pos);
                            window_action.send(WindowAction::SetRect(*surface, geo.geometry));
                        }
                        GrabEventKind::PointerAxis(_, _) | GrabEventKind::Keyboard(_, _) => {
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
                    let Ok(mut geo) = surface_query.get_component_mut::<Geometry>(*surface) else {
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
                            window_action.send(WindowAction::SetRect(*surface, geo.geometry));
                        }
                        GrabEventKind::PointerAxis(_, _)
                        | GrabEventKind::Keyboard(_, _)
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
        app.add_systems(
            PreUpdate,
            on_grab_event
                .run_if(on_event::<GrabEvent>())
                .in_set(DWayServerSet::GrabInput),
        );
    }
}
