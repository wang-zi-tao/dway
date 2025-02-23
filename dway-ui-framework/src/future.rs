use std::sync::{mpsc, Arc};

use bevy::tasks::Task;
use bevy_relationship::reexport::SmallVec;
use tokio::{sync::oneshot, task::JoinHandle};

use crate::prelude::*;

pub struct AsyncContextReceiver {
    command_receiver: mpsc::Receiver<Box<dyn FnOnce(&mut World) + Send + Sync + 'static>>,
    async_context: Arc<AsyncContextReceiver>,
}

impl AsyncContextReceiver {
    pub fn schedule(world: &mut World) {
        let this = world.non_send_resource::<Self>();
        let new_commands = this
            .command_receiver
            .try_iter()
            .collect::<SmallVec<[_; 8]>>();
        for command in new_commands {
            command(world);
        }
        world.flush();
    }

    pub fn async_context(&self) -> Arc<AsyncContextReceiver> {
        self.async_context.clone()
    }
}

#[derive(Clone)]
pub struct AsyncWorldContext {
    command_sender: mpsc::Sender<Box<dyn FnOnce(&mut World) + Send + Sync + 'static>>,
}

impl AsyncWorldContext {
    pub async fn wait_observer<E: Event, F>(&self, entity: Entity, filter: F) -> bool
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        let (tx, rx) = oneshot::channel::<()>();
        if self
            .command_sender
            .send(Box::new(move |world: &mut World| {
                let mut tx = Some(tx);
                world
                    .commands()
                    .entity(entity)
                    .observe(move |trigger: Trigger<E, ()>| {
                        if filter(trigger.event()) {
                            if let Some(tx) = tx.take() {
                                let _ = tx.send(());
                            }
                        }
                    });
            }))
            .is_err()
        {
            return false;
        };
        rx.await.is_ok()
    }

    pub async fn listen_observer<E: Event + Clone, F>(
        &self,
        entity: Entity,
    ) -> tokio::sync::mpsc::Receiver<E>
    where
        F: Fn(&E) -> bool + Send + Sync + 'static,
    {
        let (tx, rx) = tokio::sync::mpsc::channel::<E>(16);
        let _ = self.command_sender.send(Box::new(move |world: &mut World| {
            world
                .commands()
                .entity(entity)
                .observe(move |trigger: Trigger<E, ()>| {
                    let _ = tx.send(trigger.event().clone());
                });
        }));
        rx
    }

    pub async fn run_in_world<R, F>(&self, func: F) -> Option<R>
    where
        F: Fn(&mut World) -> R + Send + Sync + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<R>();
        if self
            .command_sender
            .send(Box::new(move |world: &mut World| {
                let ret = func(world);
                let _ = tx.send(ret);
            }))
            .is_err()
        {
            return None;
        };
        rx.await.ok()
    }
}

#[derive(Component, Default)]
pub struct TaskStub {
    tasks: Vec<Task<()>>,
}
