use std::{
    any::TypeId,
    cell::RefCell,
    rc::Rc,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Weak,
    },
};

use bevy::{
    ecs::{component::ComponentId, world::EntityMut},
    prelude::*,
    utils::HashMap,
};
use smallvec::SmallVec;

use crate::{ConnectableMut, Relationship, ReverseRelationship};

pub type DisconnectPeerFn = fn(&mut World, Entity, Entity);

#[derive(Clone)]
pub struct ConnectionEventSender {
    pub this_entity: Entity,
    pub sender: Option<Arc<Sender<(DisconnectPeerFn, Entity, Entity)>>>,
}
impl std::fmt::Debug for ConnectionEventSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionEventSender").finish()
    }
}
impl Default for ConnectionEventSender {
    fn default() -> Self {
        Self {
            this_entity: Entity::PLACEHOLDER,
            sender: Default::default(),
        }
    }
}
impl ConnectionEventSender {
    pub fn send<T: ConnectableMut>(&self, peer_entity: Entity) {
        if let Some(sender) = &self.sender {
            let _ = sender.send((remove_peer::<T>, self.this_entity, peer_entity));
        }
    }

    pub fn inited(&self)->bool{
        self.sender.is_some()
    }
}

fn remove_peer<C: ConnectableMut>(world: &mut World, this_entity: Entity, peer_entity: Entity) {
    if let Some(mut component) = world.get_mut::<C>(peer_entity) {
        component.disconnect(this_entity);
    };
}

pub struct ConnectionEventReceiver {
    pub receiver: Arc<Receiver<(DisconnectPeerFn, Entity, Entity)>>,
    pub sender: Arc< Sender<(DisconnectPeerFn, Entity, Entity)> >,
}
impl ConnectionEventReceiver {
    pub fn get_sender(&self,this_entity:Entity) -> ConnectionEventSender {
        ConnectionEventSender{this_entity,sender:Some(self.sender.clone())}
    }
}
impl Default for ConnectionEventReceiver {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            receiver: Arc::new(rx),
            sender: Arc::new(tx),
        }
    }
}

pub fn apply_disconnection(world: &mut World) {
    let receiver = world
        .non_send_resource_mut::<ConnectionEventReceiver>()
        .receiver
        .clone();

    for (func, entity, peer) in receiver.try_iter() {
        func(world, entity, peer);
    }
}
