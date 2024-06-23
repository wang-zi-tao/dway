use bevy::ecs::system::SystemParam;

use crate::prelude::*;

#[derive(Resource)]
pub struct DisplayDelegate(pub GlobalId);

#[derive(SystemParam)]
pub struct DisplayListQuery<'w, 's> {
    pub display_query: Query<'w, 's, &'static mut DWayServer>,
}

impl<'w, 's> DisplayListQuery<'w, 's> {
    pub fn get_handle_list(&self) -> DisplayHandleList {
        DisplayHandleList(Vec::from_iter(
            self.display_query.iter().map(|display| display.handle()),
        ))
    }
}

#[derive(Default, Deref, DerefMut)]
pub struct DisplayHandleList(pub Vec<DisplayHandle>);

impl DisplayHandleList {
    pub fn flush(&mut self) {
        for handle in self.iter_mut() {
            if let Err(e) = handle.flush_clients() {
                error!("failed to flush wayland client: {e}");
            };
        }
    }
}
