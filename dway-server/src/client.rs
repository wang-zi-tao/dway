use bevy::utils::hashbrown::HashMap;
use wayland_backend::server::ClientId;

use crate::prelude::*;

#[derive(Resource)]
pub struct ClientIndex(pub HashMap<ClientId,Entity>);

#[derive(Component)]
pub struct Client{
    pub raw: wayland_server::Client,
}
