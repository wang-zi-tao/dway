use bevy::prelude::*;
use bevy::utils::HashMap;
use kayak_ui::prelude::*;

#[derive(Debug, Clone, Resource, Reflect, FromReflect)]
pub struct Theme {
    pub class_map: HashMap<String, KStyle>,
    pub widget_map: HashMap<String, KStyle>,
    pub colors: HashMap<String, Color>,
}
