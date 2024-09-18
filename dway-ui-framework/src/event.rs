use std::{
    any::{type_name, Any, TypeId},
    sync::atomic::{AtomicBool, Ordering},
};

use bevy::{
    ecs::system::{EntityCommands, IntoObserverSystem, SystemState},
    reflect::List,
    utils::{hashbrown::hash_map::Entry, HashMap},
};
use bevy_relationship::reexport::SmallVec;

use crate::prelude::*;

#[bevy_trait_query::queryable]
pub trait EventDispatch<E> {
    fn on_event(&self, commands: EntityCommands, event: E);
}

#[derive(Clone, Debug, Event)]
pub struct UiClickEvent;

#[derive(Clone, Debug, Event)]
pub struct UiDataEvent<T>(T);

#[derive(Clone, Debug, PartialEq, Eq, Event)]
pub enum UiNodeAppearEvent {
    Appear,
    Disappear,
}

#[derive(Event, Debug)]
pub struct DespawnLaterEvent {
    pub entity: Entity,
    pub cancel: AtomicBool,
}

impl DespawnLaterEvent {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            cancel: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
}

impl UiNodeAppearEvent {
    pub fn appear(&self) -> bool {
        match self {
            UiNodeAppearEvent::Appear => true,
            UiNodeAppearEvent::Disappear => false,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct UiEvent<E> {
    receiver: Entity,
    sender: Entity,
    event: E,
}

impl<E> UiEvent<E> {
    pub fn sender(&self) -> Entity {
        self.sender
    }

    pub fn receiver(&self) -> Entity {
        self.receiver
    }

    pub fn event(&self) -> &E {
        &self.event
    }
}

impl<E: std::ops::Deref> std::ops::Deref for UiEvent<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

pub enum EventReceiver<I> {
    SystemId(Entity, SystemId<UiEvent<I>>),
    Trigger(Entity),
    Trait(Entity),
    Lambda(Box<dyn Fn(Entity, &I, &mut Commands)>),
}

#[derive(Component)]
pub struct EventDispatcher<E> {
    pub callbacks: SmallVec<[EventReceiver<E>; 2]>,
    pub run_global_triggers: bool,
    pub run_sender_trigger: bool,
    pub run_sender_traits: bool,
}

impl<E: Clone + Send + Sync + 'static> EventDispatcher<E> {
    pub fn send(&self, event: E, sender: Entity, commands: &mut Commands) {
        let mut trait_entitys: SmallVec<[Entity; 8]> = SmallVec::new();
        for receiver in self.callbacks.iter() {
            match receiver {
                EventReceiver::SystemId(entity, system) => {
                    commands.run_system_with_input(
                        *system,
                        UiEvent {
                            receiver: *entity,
                            event: event.clone(),
                            sender,
                        },
                    );
                }
                EventReceiver::Trigger(receiver) => {
                    commands.trigger_targets(
                        UiEvent {
                            receiver: *receiver,
                            event: event.clone(),
                            sender,
                        },
                        *receiver,
                    );
                }
                EventReceiver::Trait(entity) => {
                    trait_entitys.push(*entity);
                }
                EventReceiver::Lambda(f) => {
                    f(sender, &event, commands);
                }
            }
        }

        {
            let event = event.clone();
            if self.run_sender_traits {
                trait_entitys.push(sender);
            }
            if !trait_entitys.is_empty() {
                commands.add(move |world: &mut World| {
                    let mut system_state =
                        SystemState::<(Query<All<&dyn EventDispatch<E>>>, Commands)>::new(world);
                    let (query, mut commands) = system_state.get(world);
                    for trait_impls in trait_entitys.into_iter().filter_map(|e| query.get(e).ok()) {
                        let mut entity_commands = commands.entity(sender);
                        for impl_component in trait_impls {
                            impl_component.on_event(entity_commands.reborrow(), event.clone());
                        }
                    }
                });
            }
        }

        if self.run_sender_trigger {
            commands.trigger_targets(
                UiEvent {
                    receiver: sender,
                    event: event.clone(),
                    sender,
                },
                sender,
            );
        }

        if self.run_global_triggers {
            commands.trigger(UiEvent {
                receiver: Entity::PLACEHOLDER,
                event: event.clone(),
                sender,
            });
        }
    }
}

pub trait UiEventAppExt {
    #[deprecated]
    fn register_callback<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static;
}
impl UiEventAppExt for App {
    fn register_callback<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static,
    {
        let type_id = system.type_id();
        let system_id = self.world_mut().register_system(system);
        let mut theme = self.world_mut().resource_mut::<UiEventRegister>();
        theme.systems.insert(type_id, Box::new(system_id));
        self
    }
}

#[derive(Resource)]
pub struct UiEventRegister {
    pub systems: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    pub triggers: HashMap<TypeId, Entity>,
}

impl UiEventRegister {
    pub fn register_system<F, I, M>(&mut self, system: F, commands: &mut Commands) -> SystemId<I>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: Send + 'static,
    {
        let type_id = system.type_id();
        match self.systems.entry(type_id) {
            Entry::Occupied(o) => *o.get().downcast_ref().unwrap(),
            Entry::Vacant(v) => {
                let system_id = commands.register_one_shot_system(system);
                v.insert(Box::new(system_id));
                system_id
            }
        }
    }

    pub fn add_trigger<F, E: Event, B: Bundle, M>(
        &mut self,
        system: F,
        commands: &mut Commands,
        entity: Entity,
    ) where
        F: IntoObserverSystem<E, B, M>,
    {
        let type_id = system.type_id();
        match self.triggers.entry(type_id) {
            Entry::Occupied(o) => {
                commands.entity(*o.get()).add(move |mut c: EntityWorldMut| {
                    let mut observer = c.get_mut::<Observer<E, B>>().unwrap();
                    observer.watch_entity(entity);
                });
            }
            Entry::Vacant(v) => {
                let trigger = commands
                    .spawn(Observer::new(system).with_entity(entity))
                    .id();
                v.insert(trigger);
            }
        }
    }

    pub fn system<F, I, M>(&self, system: F) -> SystemId<I, ()>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: 'static,
    {
        let Some(callback) = self.systems.get(&system.type_id()) else {
            panic!(
                "system is not registered: {system}
note: add code
```
use dway_ui_framework::event::UiEventAppExt;
app.register_callback({system});
``` to the plugin to register the system",
                system = type_name::<F>()
            );
        };
        *callback.as_ref().downcast_ref().unwrap()
    }
}

pub fn on_despawn_later_event(mut events: EventReader<DespawnLaterEvent>, mut commands: Commands) {
    for e in events.read() {
        if !e.is_cancelled() {
            commands.entity(e.entity).despawn_recursive();
        }
    }
}
