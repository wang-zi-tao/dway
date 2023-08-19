use wayland_backend::server::{ClientId, DisconnectReason};

use crate::prelude::*;

#[derive(Component)]
pub struct Client {
    pub raw: wayland_server::Client,
}

impl Client {
    pub fn new(raw: wayland_server::Client) -> Self {
        Self { raw }
    }
}
#[derive(Debug)]
pub struct ClientData {
    pub entity: Entity,
}

impl ClientData {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}
impl wayland_backend::server::ClientData for ClientData {
    /// Notification that a client was initialized
    fn initialized(&self, client: ClientId) {
        info!(?client, "client connected");
    }
    /// Notification that a client is disconnected
    fn disconnected(&self, client: ClientId, reason: DisconnectReason) {
        info!(entity=?self.entity, ?client, ?reason, "client disconnected");
    }
}
