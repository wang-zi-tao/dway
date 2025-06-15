use bitflags::bitflags;

use crate::{geometry::Geometry, prelude::*, util::rect::IRect};

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
    pub struct ResizeEdges: u8 {
        const TOP =     0b00000001;
        const BUTTOM =  0b00000010;
        const LEFT =    0b00000100;
        const RIGHT =   0b00001000;
    }
}

#[derive(Event, Debug, Reflect)]
pub enum StartGrab {
    Move {
        surface: Entity,
        seat: Entity,
        serial: Option<u32>,
        mouse_pos: IVec2,
        geometry: Geometry,
    },
    Resizing {
        surface: Entity,
        seat: Entity,
        #[reflect(ignore)]
        edges: ResizeEdges,
        serial: Option<u32>,
        geometry: Geometry,
    },
    Drag {
        surface: Entity,
        seat: Entity,
        data_device: Entity,
        icon: Option<Entity>,
    },
}

#[derive(Component, Debug, Default, Reflect)]
pub struct WlSurfacePointerState {
    pub mouse_pos: IVec2,
}

impl WlSurfacePointerState {
}

pub struct GrabPlugin;

impl Plugin for GrabPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StartGrab>();
        app.register_type::<WlSurfacePointerState>();
    }
}
