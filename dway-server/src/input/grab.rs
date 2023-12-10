use bitflags::bitflags;

use crate::prelude::*;

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
    Move {
        seat: Entity,
        serial: u32,
    },
    Resizing {
        seat: Entity,
        edges: ResizeEdges,
        serial: u32,
    },
}

#[derive(Component, Debug, Default, Reflect)]
pub struct WlSurfacePointerState {
    pub is_clicked: bool,
    pub mouse_pos: IVec2,
    #[reflect(ignore)]
    pub grab: Option<Box<SurfaceGrabKind>>,
}
impl WlSurfacePointerState {
    pub fn is_grabed(&self) -> bool {
        self.is_clicked || self.grab.is_some()
    }
    pub fn enabled(&self) -> bool {
        self.grab.is_none()
    }
}
