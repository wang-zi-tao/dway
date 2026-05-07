use x11rb::protocol::xproto::Screen;

use super::window::XWindow;
use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
};

#[derive(Component, Debug)]
pub struct XScreen {
    pub raw: Screen,
}

#[derive(Bundle)]
pub struct XScreenBundle {
    pub window: XWindow,
    pub screen: XScreen,
    pub geometry: Geometry,
    pub global_geometry: GlobalGeometry,
}
