use bevy::{
    input::{
        mouse::{MouseButtonInput, MouseWheel},
        ButtonState,
    },
    math::DVec2,
    prelude::Component,
};
use bitflags::bitflags;
use kayak_ui::prelude::OnEvent;
use wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge;

use crate::{
    geometry::{Geometry, GeometryPlugin, GlobalGeometry},
    prelude::*,
    schedule::DWayServerSet,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, toplevel::XdgToplevel, XdgSurface},
};

use super::pointer::WlPointer;

pub struct PointerMoveGrabEvent(pub Entity, pub Vec2);
pub struct PointerButtonGrabEvent(pub Entity, pub MouseButtonInput, pub DVec2);
pub struct PointerAxisGrabEvent(pub Entity, pub MouseWheel, pub DVec2);
pub struct KeyboardGrabEvent(pub Entity);

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
    mut surface_query: Query<(&WlSurface, &GlobalGeometry, Option<&XdgPopup>)>,
) {
    for PointerButtonGrabEvent(entity, event, position) in event.iter() {
        if let Ok((mut pointer, mut grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::ButtonDown { surface } => {
                    let Ok((surface, rect, popup)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.button(event, surface, *position - rect.pos().as_dvec2());
                    match event.state {
                        ButtonState::Pressed => {
                            pointer.grab(surface);
                        }
                        ButtonState::Released => {
                            *grab = PointerGrab::None;
                            pointer.unset_grab();
                        }
                    }
                }

                PointerGrab::OnPopup { surface, .. } => {
                    let Ok((surface, rect, popup)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.button(event, surface, *position - rect.pos().as_dvec2());
                    match event.state {
                        ButtonState::Pressed => {
                            pointer.grab(surface);
                        }
                        ButtonState::Released => {
                            pointer.unset_grab();
                        }
                    }
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
        app.register_type::<PointerGrab>();
        app.register_type::<KeyboardGrab>();
        app.add_event::<PointerMoveGrabEvent>();
        app.add_event::<PointerButtonGrabEvent>();
        app.add_event::<PointerAxisGrabEvent>();
        app.add_event::<KeyboardGrabEvent>();
        app.add_systems(
            (
                on_mouse_move.run_if(on_event::<PointerMoveGrabEvent>()),
                on_mouse_button.run_if(on_event::<PointerButtonGrabEvent>()),
                on_mouse_axis.run_if(on_event::<PointerAxisGrabEvent>()),
            )
                .in_set(DWayServerSet::GrabInput),
        );
    }
}
