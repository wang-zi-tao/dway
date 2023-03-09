use std::time::SystemTime;

use bevy::{
    input::{
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseWheel},
    },
    prelude::{default, Component, IVec2, Rect, Vec2},
    reflect::Reflect,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Reflect, Default, Component, PartialEq, Eq)]
pub enum WindowState {
    #[default]
    Normal,
    Minimized,
    Maximized,
    FullScreen,
}

#[derive(Debug)]
pub struct WindowMessage {
    pub uuid: Uuid,
    pub time: SystemTime,
    pub data: WindowMessageKind,
}

pub struct ImageBuffer(pub Vec2, pub Vec<u8>);
impl std::fmt::Debug for ImageBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ImageBuffer")
            .field(&self.0)
            .field(&self.1.len())
            .finish()
    }
}
#[derive(Debug)]
pub enum WindowMessageKind {
    Create {
        pos: Vec2,
        size: Vec2,
    },
    Destroy,
    UpdateImage {
        geo: Rect,
        bbox: Rect,
        image: ImageBuffer,
    },
    Move(IVec2),
    MoveRequest,
    ResizeRequest {
        top: bool,
        bottom: bool,
        left: bool,
        right: bool,
    },
    SetRect(Rect),
    Normal,
    Minimized,
    Maximized,
    Unmaximized,
    Unminimized,
    FullScreen,
    UnFullScreen,
    Sync {
        state: WindowState,
        pos: Vec2,
        buffer: ImageBuffer,
        title: String,
    },
    // relative to bbox
    MouseMove(Vec2),
    MouseButton(MouseButtonInput),
    MouseWheel(MouseWheel),
    KeyboardInput(KeyboardInput),
}
