use bevy::{
    input::mouse::{MouseButtonInput, MouseWheel},
    math::DVec2,
    prelude::Component,
};
use bitflags::bitflags;
use kayak_ui::prelude::OnEvent;

use crate::{
    geometry::{GeometryPlugin, GlobalGeometry},
    prelude::*,
    schedule::DWayServerSet,
    wl::surface::WlSurface,
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
    },
    Resizing {
        surface: Entity,
        #[reflect(ignore)]
        edges: ResizeEdges,
    },
}

pub fn on_mouse_move(
    mut event: EventReader<PointerMoveGrabEvent>,
    mut pointer_query: Query<(&mut WlPointer, &mut PointerGrab)>,
    mut surface_query: Query<(&WlSurface, &GlobalGeometry)>,
) {
    for PointerMoveGrabEvent(entity, pos) in event.iter() {
        if let Ok((mut pointer, grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::OnPopup { surface, serial } => {
                    let Ok((surface, rect)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.move_cursor(surface, *pos - rect.pos().as_vec2());
                }
                PointerGrab::ButtonDown { surface } => todo!(),
                PointerGrab::Moving { surface } => todo!(),
                PointerGrab::Resizing { surface, edges } => todo!(),
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
        if let Ok((mut pointer, grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::OnPopup { surface, serial } => {
                    let Ok((surface, rect)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.set_focus(surface, *position - rect.pos().as_dvec2());
                    pointer.button(event);
                }
                PointerGrab::ButtonDown { surface } => todo!(),
                PointerGrab::Moving { surface } => todo!(),
                PointerGrab::Resizing { surface, edges } => todo!(),
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
        if let Ok((mut pointer, grab)) = pointer_query.get_mut(*entity) {
            match &*grab {
                PointerGrab::OnPopup { surface, serial } => {
                    let Ok((surface, rect)) = surface_query.get(*surface) else {
                        continue;
                    };
                    pointer.set_focus(surface, *position - rect.pos().as_dvec2());
                    if event.x != 0.0 {
                        pointer.horizontal_asix(event.x as f64);
                    }
                    if event.y != 0.0 {
                        pointer.horizontal_asix(event.y as f64);
                    }
                }
                PointerGrab::ButtonDown { surface } => todo!(),
                PointerGrab::Moving { surface } => todo!(),
                PointerGrab::Resizing { surface, edges } => todo!(),
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
