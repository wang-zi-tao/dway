use bitflags::bitflags;
use crate::util::rect::IRect;
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

#[derive(Component, Debug, Reflect)]
pub enum SurfaceGrabKind {
    Move {
        seat: Entity,
        serial: Option<u32>,
        mouse_pos: IVec2,
    },
    Resizing {
        seat: Entity,
        #[reflect(ignore)]
        edges: ResizeEdges,
        serial: Option<u32>,
        geo: IRect,
    },
}

#[derive(Component, Debug, Default, Reflect)]
pub struct WlSurfacePointerState {
    pub is_clicked: bool,
    pub mouse_pos: IVec2,
    pub grab: Option<SurfaceGrabKind>,
}

impl WlSurfacePointerState {
    pub fn is_grabed(&self) -> bool {
        self.is_clicked || self.grab.is_some()
    }
    pub fn enabled(&self) -> bool {
        self.grab.is_none()
    }

    pub fn set_grab(&mut self, grab: SurfaceGrabKind) {
        self.grab = Some(grab);
    }
    pub fn clean_grab(&mut self) {
        self.grab = None;
    }
}
