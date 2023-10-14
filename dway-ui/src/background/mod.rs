use bevy::prelude::*;
use kayak_ui::{prelude::*, KayakUIPlugin};

#[derive(Default)]
pub struct DWayBackgroundPlugin {}
impl Plugin for DWayBackgroundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {}
}

impl KayakUIPlugin for DWayBackgroundPlugin {
    fn build(&self, _context: &mut kayak_ui::prelude::KayakRootContext) {}
}

#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayBackgroundProps {}
impl Widget for DWayBackgroundProps {}
#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayBackgroundStates {}

#[derive(Bundle)]
pub struct DWayBackgroundBundle {
    pub props: DWayBackgroundProps,
    pub styles: KStyle,
    pub computed_styles: ComputedStyles,
    pub widget_name: WidgetName,
}
impl Default for DWayBackgroundBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: DWayBackgroundProps::default().get_name(),
        }
    }
}
