pub mod apps;

use crate::prelude::*;

#[derive(Resource, Default)]
pub struct Database {}

pub struct DWayClientModelPlugin;
impl Plugin for DWayClientModelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Database>()
            .init_resource::<apps::AppListModel>();
    }
}
