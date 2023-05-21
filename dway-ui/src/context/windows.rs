use bevy::prelude::*;
use dway_server::events::CreateWindow;


#[derive(Component,Default)]
pub struct WindowsList{
    pub windows:Vec<Entity>,
}

pub fn update_window_list(
    events:EventReader<CreateWindow>,
    windows_list_query:Query<&mut WindowsList>
){

}
