use bevy_relationship::relationship;

use crate::prelude::*;

#[derive(Component, Default)]
pub struct Cursor {
    pub serial: Option<u32>,
}

impl Cursor {
    pub fn new(serial: Option<u32>) -> Self {
        Self { serial }
    }
}
relationship!(PointerHasSurface=> SurfaceRef -- PointerRef);
