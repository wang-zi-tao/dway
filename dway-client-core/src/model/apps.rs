use bevy::platform::collections::HashMap;
use indexmap::{IndexMap, IndexSet};

use crate::prelude::*;

#[derive(Deref, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct AppId(String);

impl From<String> for AppId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub struct AppInfo {
    pub appid: AppId,
    pub name: String,
    pub icon: String,
    pub hidden: bool,
}

#[derive(Resource, Default)]
pub struct AppListModel {
    pub apps: HashMap<AppId, AppInfo>,
    pub favorite_apps: IndexSet<AppId>,
    pub folder: IndexMap<String, Vec<AppId>>,
}

pub enum RuleRequest{
    AddWindow,
    AddToplevel,
    AddPopup,
}

pub enum RuleResponse{
    SetupWindow{
        pos: Option<IVec2>,
        size: Option<IVec2>,
        workspace: Option<Entity>,
        screen: Option<Entity>,
        slot: Option<Entity>,
    },
}
