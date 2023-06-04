use bevy::prelude::*;


#[derive(Component,Default)]
pub struct WindowsList{
    pub windows:Vec<Entity>,
}

pub fn update_window_list(
    // events:EventReader<Insert<XdgSurface>>,
    windows_list_query:Query<&mut WindowsList>
){
}
