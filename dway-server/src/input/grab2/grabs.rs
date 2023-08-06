use bevy::{ecs::system::SystemState, math::DVec2};

use crate::{
    geometry::GlobalGeometry,
    input::{
        grab::GrabControl,
        pointer::WlPointer,
        seat::{PointerList, SeatHasPointer},
    },
    prelude::*,
    wl::{region::WlRegion, surface::{WlSurface, SurfaceHasInputRegion}},
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, XdgSurface},
};

use super::Grab;

pub struct Focus;
impl Grab for Focus {
    fn input(
        &self,
        world: &mut World,
        target: Entity,
        event: super::SeatInputEvent,
    ) -> super::GrabControl {
        graph_query!(PointerGraph=>[
            seat=Entity,
            pointer=&'static mut WlPointer,
        ]=>{ path=seat-[SeatHasPointer]->pointer });
        graph_query!(SurfaceGraph=>[
            surface=(Entity, &'static WlSurface, &'static GlobalGeometry, Option<&'static XdgToplevel>, Option<&'static XdgPopup>,),
            region=(&'static mut WlRegion),
        ]=>{ path=surface-[SurfaceHasInputRegion]->region });
        let mut system_state = SystemState::<(PointerGraph, SurfaceGraph)>::from_world(world);
        let (mut pointer_graph, mut surface_graph) = system_state.get_mut(world);

        surface_graph.for_each_path_mut_from(
            target,
            |(surface_entity, surface, rect, toplevel, popup), region| {
                pointer_graph.for_each_path_mut_from(event.0, |_, pointer| {
                    match event.1 {
                        crate::input::grab::SeatInputEventKind::PointerMoveGrabEvent(pos) => {
                            if popup.is_none() {
                                if !rect.include_point(pos.as_ivec2())
                                    || !region.is_inside(
                                        pos.as_ivec2()
                                            - rect.geometry.pos()
                                            - surface.image_rect().pos(),
                                    )
                                {
                                    if pointer.can_focus_on(surface) {
                                        // pointer.leave();
                                    }
                                    return;
                                }
                                let relative = pos.as_ivec2()
                                    - rect.geometry.pos()
                                    - surface.image_rect().pos();
                                pointer.move_cursor(surface, relative.as_vec2());
                            }
                        }
                        crate::input::grab::SeatInputEventKind::PointerButtonGrabEvent(
                            input,
                            pos,
                        ) => {
                            pointer.button(&input, surface, pos - rect.pos().as_dvec2());
                        }
                        crate::input::grab::SeatInputEventKind::PointerAxisGrabEvent(
                            input,
                            pos,
                        ) => {
                            let acc = |x: f64| x * 20.0;
                            pointer.asix(
                                DVec2::new(-acc(input.x as f64), -acc(input.y as f64)),
                                surface,
                                pos,
                            );
                        }
                        crate::input::grab::SeatInputEventKind::KeyboardGrabEvent(_) => todo!(),
                    }
                });
            },
        );
        GrabControl::None
    }
}

pub struct ButtonGrab;
impl Grab for ButtonGrab {
    fn input(
        &self,
        world: &mut World,
        target: Entity,
        event: super::SeatInputEvent,
    ) -> super::GrabControl {
        let system_state =
            SystemState::<(Query<&PointerList>, Query<&mut WlPointer>)>::from_world(world);
        todo!()
    }
}
pub struct PopupGrab;
impl Grab for PopupGrab {
    fn input(
        &self,
        world: &mut World,
        target: Entity,
        event: super::SeatInputEvent,
    ) -> super::GrabControl {
        todo!()
    }
}
pub struct MovingGrab;
impl Grab for MovingGrab {
    fn input(
        &self,
        world: &mut World,
        target: Entity,
        event: super::SeatInputEvent,
    ) -> super::GrabControl {
        todo!()
    }
}
pub struct ResizingGrab;
impl Grab for ResizingGrab {
    fn input(
        &self,
        world: &mut World,
        target: Entity,
        event: super::SeatInputEvent,
    ) -> super::GrabControl {
        todo!()
    }
}
