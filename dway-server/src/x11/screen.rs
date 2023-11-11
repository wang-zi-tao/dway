use x11rb::protocol::xproto::Screen;

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
};

use super::window::XWindow;

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
