use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use dway_util::eventloop::PollerRawGuard;
use wayland_backend::server::{ClientId, DisconnectReason};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum ClientEvent {
    Destroyed { entity: Entity, id: ClientId },
}

#[derive(Resource, Default, Clone)]
pub struct ClientEvents {
    queue: Arc<Mutex<VecDeque<ClientEvent>>>,
}

#[derive(Component)]
pub struct Client {
    pub id: ClientId,
}

impl Client {
    pub fn new(raw: &wayland_server::Client) -> Self {
        Self { id: raw.id() }
    }
}
#[derive(Debug)]
pub struct ClientData {
    pub entity: Entity,
    queue: Arc<Mutex<VecDeque<ClientEvent>>>,
    poller_guard: PollerRawGuard,
}

impl ClientData {
    pub fn new(entity: Entity, events: &ClientEvents, poller_guard: PollerRawGuard) -> Self {
        Self {
            entity,
            queue: events.queue.clone(),
            poller_guard,
        }
    }
}
impl wayland_backend::server::ClientData for ClientData {
    fn initialized(&self, client: ClientId) {
        info!(entity=?self.entity, ?client, "client connected");
    }

    fn disconnected(&self, client: ClientId, reason: DisconnectReason) {
        info!(entity=?self.entity, ?client, ?reason, "client disconnected");
        self.queue
            .lock()
            .unwrap()
            .push_back(ClientEvent::Destroyed {
                entity: self.entity,
                id: client,
            });
    }
}
impl Drop for ClientData {
    fn drop(&mut self) {
        debug!(entity=?self.entity, "client droped");
    }
}

pub fn clean_client(
    events: Res<ClientEvents>,
    mut events_writer: EventWriter<Destroy<Client>>,
    mut commands: Commands,
) {
    let mut queue = events.queue.lock().unwrap();
    while let Some(event) = queue.pop_front() {
        match event {
            ClientEvent::Destroyed { entity, .. } => {
                if let Ok(mut c) = commands.get_entity(entity) {
                    c.despawn()
                }
                events_writer.send(Destroy::new(entity));
            }
        }
    }
}

pub struct ClientPlugin;
impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, clean_client.in_set(DWayServerSet::Clean));
        app.add_event::<Insert<Client>>();
        app.add_event::<Destroy<Client>>();
        app.init_resource::<ClientEvents>();
    }
}
