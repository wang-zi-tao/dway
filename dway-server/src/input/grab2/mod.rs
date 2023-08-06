pub mod grabs;

use std::sync::Arc;

use bevy::{
    ecs::{event::ManualEventReader, system::SystemState},
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::Component,
};
use bevy_relationship::reexport::SmallVec;
use bitflags::bitflags;
use kayak_ui::prelude::OnEvent;
use wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;

use crate::{
    geometry::{Geometry, GeometryPlugin, GlobalGeometry},
    prelude::*,
    schedule::DWayServerSet,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{toplevel::XdgToplevel, XdgSurface},
};
pub enum GrabControl {
    None,
    Remove,
}

pub trait Grab: Send + Sync {
    fn input(&self, world: &mut World, target: Entity, event: SeatInputEvent) -> GrabControl;
}

relationship!(SeatHasGrab=>GrabTargetList-<SeatRef);

#[derive(Component, Reflect, FromReflect, Default)]
pub struct GrabTarget {
    pub inner: SmallVec<[Arc<dyn Grab>; 1]>,
}

use super::{
    pointer::WlPointer,
    seat::{PopupGrabBy, WlSeat},
};

pub enum SeatInputEventKind {
    PointerMoveGrabEvent(Vec2),
    PointerButtonGrabEvent(MouseButtonInput, DVec2),
    PointerAxisGrabEvent(MouseWheel, DVec2),
    KeyboardGrabEvent(KeyboardInput),
}
pub struct SeatInputEvent(pub Entity, pub SeatInputEventKind);

pub fn on_input(world: &mut World) {
    let mut inputs = Vec::new();
    {
        let mut event_reader = ManualEventReader::<SeatInputEvent>::from_world(world);
        let mut system_state: SystemState<(
            Query<(&WlSeat, &mut GrabTargetList)>,
            Query<&mut GrabTarget>,
        )> = SystemState::from_world(world);
        let (mut seat_query, mut target_query) = system_state.get_mut(world);
        for event in event_reader.iter(world.resource()) {
            if let Ok((seat, targets)) = seat_query.get_mut(event.0) {
                for target_entity in targets.iter().rev() {
                    if let Ok(mut target) = target_query.get_mut(target_entity) {
                        if let Some(grab) = target.inner.last_mut() {
                            inputs.push((target_entity, *event, Arc::downgrade(grab)));
                        }
                    }
                }
            }
            system_state.apply(world);
        }
    }
    let mut controls = Vec::new();
    {
        for (target_entity, event, grab) in inputs {
            let Some(grab) = grab.upgrade() else {
                continue;
            };
            let control = grab.input(world, target_entity, event);
            match control {
                GrabControl::Remove => controls.push((event.0, target_entity, control)),
                GrabControl::None => {}
            }
        }
    }
    if controls.len() > 0 {
        let mut system_state: SystemState<(
            Query<(&mut GrabTarget, &mut SeatRef)>,
            Query<&mut GrabTargetList>,
            Commands,
        )> = SystemState::from_world(world);
        let (mut grab_query, mut seat_query, mut commands) = system_state.get_mut(world);
        for (seat_entity, target_entity, control) in controls {
            match control {
                GrabControl::None => {}
                GrabControl::Remove => {
                    if let Ok(mut grab_list) = seat_query.get_mut(seat_entity) {
                        grab_list.pop();
                        if let Ok((mut grabs, mut seat_ref)) = grab_query.get_mut(target_entity) {
                            grabs.inner.pop();
                            if grab_list.len() == 0 {
                                seat_ref.take();
                            }
                        }
                    };
                }
            }
        }
    }
}

#[derive(Component, Debug, Default, Reflect, FromReflect)]
#[reflect(Debug)]
pub enum KeyboardGrab {
    #[default]
    None,
    KeyboardGrab {
        entity: Entity,
        serial: i32,
    },
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
pub enum PointerGrab {
    #[default]
    None,
    OnPopup {
        surface: Entity,
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

pub fn on_mouse_move(
    mut event: EventReader<PointerMoveGrabEvent>,
    mut pointer_query: Query<(&mut WlPointer, &mut PointerGrab, &Geometry, &GlobalGeometry)>,
    mut surface_query: Query<
        (
            &WlSurface,
            &mut Geometry,
            &GlobalGeometry,
            Option<&XdgToplevel>,
            Option<&XdgSurface>,
        ),
        Without<WlPointer>,
    >,
) {
    for PointerMoveGrabEvent(entity, pos) in event.iter() {
        if let Ok((mut pointer, grab, pointer_rect, pointer_global_rect)) =
            pointer_query.get_mut(*entity)
        {
            match &*grab {
                PointerGrab::ButtonDown { surface } | PointerGrab::OnPopup { surface, .. } => {
                    let Ok((surface, rect, global, ..)) = surface_query.get(*surface) else {
                        continue;
                    };
                    let relative =
                        pos.as_ivec2() - global.geometry.pos() - surface.image_rect().pos();
                    pointer.move_cursor(surface, relative.as_vec2());
                }
                PointerGrab::Moving {
                    surface,
                    serial,
                    relative,
                } => {
                    let Ok((surface, mut rect, global, ..)) = surface_query.get_mut(*surface)
                    else {
                        continue;
                    };
                    rect.set_pos(*relative + pointer_rect.pos());
                }
                PointerGrab::Resizing {
                    surface,
                    edges,
                    serial,
                    relative,
                    origin_rect,
                } => {
                    let Ok((surface, mut rect, global, toplevel, xdg_surface)) =
                        surface_query.get_mut(*surface)
                    else {
                        continue;
                    };
                    let top_left = *relative + pointer_rect.pos();
                    let buttom_right = top_left + origin_rect.size();
                    if edges.contains(ResizeEdges::LEFT) {
                        rect.min.x = top_left.x;
                    }
                    if edges.contains(ResizeEdges::TOP) {
                        rect.min.y = top_left.y;
                    }
                    if edges.contains(ResizeEdges::RIGHT) {
                        rect.max.x = buttom_right.x;
                    }
                    if edges.contains(ResizeEdges::BUTTOM) {
                        rect.max.y = buttom_right.y;
                    }
                    toplevel.map(|t| t.resize(rect.size()));
                    xdg_surface.map(|s| s.configure());
                }
                _ => {}
            }
        }
    }
}
pub fn on_mouse_button(
    mut event: EventReader<PointerButtonGrabEvent>,
    mut pointer_query: Query<(&mut WlPointer, &mut PointerGrab)>,
    mut surface_query: Query<(&WlSurface, &GlobalGeometry)>,
) {
    for PointerButtonGrabEvent(entity, event, position) in event.iter() {
        if let Ok((mut pointer, mut grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::ButtonDown { surface } | PointerGrab::OnPopup { surface, .. } => {
                    let Ok((surface, rect)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.button(event, surface, *position - rect.pos().as_dvec2());
                }
                PointerGrab::Moving { .. } => {
                    *grab = PointerGrab::None;
                    pointer.unset_grab();
                    info!("stop moving");
                }
                PointerGrab::Resizing { .. } => {
                    *grab = PointerGrab::None;
                    pointer.unset_grab();
                    info!("stop resizing");
                }
                _ => {}
            }
            match &*grab {
                PointerGrab::ButtonDown { surface } => {
                    if event.state == ButtonState::Released {
                        *grab = PointerGrab::None;
                        pointer.unset_grab();
                    }
                }
                _ => {}
            }
        }
    }
}
pub fn on_mouse_axis(
    mut event: EventReader<PointerAxisGrabEvent>,
    mut pointer_query: Query<(&mut WlPointer, &mut PointerGrab)>,
    mut surface_query: Query<(&WlSurface, &GlobalGeometry)>,
) {
    for PointerAxisGrabEvent(entity, event, position) in event.iter() {
        if let Ok((mut pointer, mut grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::ButtonDown { surface } | PointerGrab::OnPopup { surface, .. } => {
                    let Ok((surface, rect)) = surface_query.get(*surface) else {
                        continue;
                    };
                    let acc = |x: f64| x * 20.0;
                    pointer.asix(
                        DVec2::new(-acc(event.x as f64), -acc(event.y as f64)),
                        surface,
                        *position,
                    );
                }
                _ => {}
            }
        }
    }
}

pub struct GrabPlugin;
impl Plugin for GrabPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<SeatHasGrab>();
        app.add_systems(
            (
                on_input.run_if(on_event::<SeatInputEvent>()),
            )
                .in_set(DWayServerSet::GrabInput),
        );
    }
}
