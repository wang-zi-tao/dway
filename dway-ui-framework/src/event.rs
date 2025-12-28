use std::{
    any::{type_name, Any, TypeId},
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

use bevy::{
    ecs::{
        event::{EntityTrigger, Trigger}, lifecycle::HookContext, system::{EntityCommand, EntityCommands, IntoObserverSystem, SystemState}
    },
    platform::collections::{HashMap, hash_map::Entry},
    reflect::List,
};
use bevy_relationship::reexport::{Mutable, SmallVec, StorageType};

use crate::prelude::*;

#[bevy_trait_query::queryable]
pub trait EventReceiver<E> {
    fn on_event(&self, commands: EntityCommands, event: E);
}

#[macro_export]
macro_rules! impl_event_receiver {
    (impl EventReceiver<$event:ty> for $receiver:ty => $system:ident) => {
        impl dway_ui_framework::event::EventReceiver<$event> for $receiver {
            fn on_event(&self, mut commands: EntityCommands, event: $event) {
                let entity = commands.id();
                let system = $system;
                commands.queue(move |entity_world_mut: EntityWorldMut<'_>| {
                    let _ = entity_world_mut
                        .into_world_mut()
                        .run_system_cached_with(system, UiEvent::new(entity, entity, event));
                });
            }
        }
    };
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

#[derive(Message, Debug)]
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

#[derive(Debug, Clone)]
pub struct UiEvent<E> {
    receiver: Entity,
    sender: Entity,
    event: E,
}

impl<E: Send + Sync + 'static> Event for UiEvent<E> {
    type Trigger<'a> = EntityTrigger;
}

impl<E: Send + Sync + 'static> EntityEvent for UiEvent<E> {
    fn event_target(&self) -> Entity {
        self.receiver
    }

    fn event_target_mut(&mut self) -> &mut Entity {
        &mut self.receiver
    }
}

impl<E> UiEvent<E> {
    pub fn new(receiver: Entity, sender: Entity, event: E) -> Self {
        Self {
            receiver,
            sender,
            event,
        }
    }

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

impl<E> SystemInput for UiEvent<E> {
    type Inner<'i> = Self;
    type Param<'i> = Self;

    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

impl<E> std::ops::Deref for UiEvent<E> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}

pub enum EventReceiverKind<I> {
    SystemId(Option<Entity>, SystemId<UiEvent<I>>),
    Trigger(Entity),
    Trait(Entity),
    Lambda(Box<dyn Fn(Entity, &I, &mut Commands) + Send + Sync + 'static>),
}

#[derive(SmartDefault)]
pub struct EventDispatcher<E: Clone + Send + Sync + 'static> {
    pub callbacks: Vec<EventReceiverKind<E>>,
    #[default(Entity::PLACEHOLDER)]
    this_entity: Entity,
    #[default(false)]
    pub run_global_triggers: bool,
    #[default(true)]
    pub run_sender_trigger: bool,
    #[default(true)]
    pub run_sender_traits: bool,
}

impl<E: Clone + Send + Sync + 'static> EventDispatcher<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, receiver: Entity, system: SystemId<UiEvent<E>>) -> &mut Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks
            .push(EventReceiverKind::SystemId(Some(receiver), system));
        self
    }

    pub fn add_system_to_this(&mut self, system: SystemId<UiEvent<E>>) -> &mut Self {
        self.callbacks
            .push(EventReceiverKind::SystemId(Some(self.this_entity), system));
        self
    }

    pub fn add_systems(&mut self, systems: &[(Entity, SystemId<UiEvent<E>>)]) -> &mut Self {
        for (receiver, system) in systems {
            assert!(*receiver != Entity::PLACEHOLDER);
            self.callbacks
                .push(EventReceiverKind::SystemId(Some(*receiver), *system));
        }
        self
    }

    pub fn add_trigger(&mut self, receiver: Entity) -> &mut Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks.push(EventReceiverKind::Trigger(receiver));
        self
    }

    pub fn add_trait_callback(&mut self, receiver: Entity) -> &mut Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks.push(EventReceiverKind::Trait(receiver));
        self
    }

    pub fn with_system_to_this(mut self, system: SystemId<UiEvent<E>>) -> Self {
        self.callbacks
            .push(EventReceiverKind::SystemId(None, system));
        self
    }

    pub fn with_system(mut self, receiver: Entity, system: SystemId<UiEvent<E>>) -> Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks
            .push(EventReceiverKind::SystemId(Some(receiver), system));
        self
    }

    pub fn with_systems(mut self, systems: &[(Entity, SystemId<UiEvent<E>>)]) -> Self {
        for (receiver, system) in systems {
            assert!(*receiver != Entity::PLACEHOLDER);
            self.callbacks
                .push(EventReceiverKind::SystemId(Some(*receiver), *system));
        }
        self
    }

    pub fn with_trigger(mut self, receiver: Entity) -> Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks.push(EventReceiverKind::Trigger(receiver));
        self
    }

    pub fn with_trait(mut self, receiver: Entity) -> Self {
        assert!(receiver != Entity::PLACEHOLDER);
        self.callbacks.push(EventReceiverKind::Trait(receiver));
        self
    }

    pub fn with_lambda<F>(mut self, f: F) -> Self
    where
        F: Fn(Entity, &E, &mut Commands) + Send + Sync + 'static,
    {
        self.callbacks.push(EventReceiverKind::Lambda(Box::new(f)));
        self
    }

    pub fn with_global_triggers(mut self) -> Self {
        self.run_global_triggers = true;
        self
    }

    pub fn with_sender_trigger(mut self) -> Self {
        self.run_sender_trigger = true;
        self
    }

    pub fn with_sender_traits(mut self) -> Self {
        self.run_sender_traits = true;
        self
    }

    pub fn new_with_system(receiver: Entity, system: SystemId<UiEvent<E>>) -> Self {
        Self::default().with_system(receiver, system)
    }
}

impl<E: Clone + Send + Sync + 'static> Component for EventDispatcher<E> {
    type Mutability = Mutable;

    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn on_insert() -> Option<ComponentHook> {
        Some(|mut world, context: HookContext| {
            let entity = context.entity;
            let mut dispatcher = world.get_mut::<EventDispatcher<E>>(entity).unwrap();
            dispatcher.this_entity = entity;
        })
    }
}

pub fn send_trait_event<E: Clone + Send + Sync + 'static>(mut commands: EntityCommands, event: E) {
    commands.queue(move |entity: EntityWorldMut| {
        let entity_id = entity.id();
        let world = entity.into_world_mut();
        let mut system_state =
            SystemState::<(Query<All<&dyn EventReceiver<E>>>, Commands)>::new(world);
        let (query, mut commands) = system_state.get(world);
        if let Ok(trait_impls) = query.get(entity_id) {
            let mut entity_commands = commands.entity(entity_id);
            for impl_component in trait_impls {
                impl_component.on_event(entity_commands.reborrow(), event.clone());
            }
        }

        system_state.apply(world);
    });
}

impl<E: Clone + Send + Sync + 'static> EventDispatcher<E> {
    pub fn send(&self, event: E, commands: &mut Commands) {
        let sender = self.this_entity;
        let mut trait_entitys: SmallVec<[Entity; 8]> = SmallVec::new();
        for receiver in self.callbacks.iter() {
            match receiver {
                EventReceiverKind::SystemId(receiver, system) => {
                    if let Some(receiver) = receiver {
                        commands.run_system_with(
                            *system,
                            UiEvent {
                                receiver: *receiver,
                                event: event.clone(),
                                sender,
                            },
                        );
                    } else {
                        commands.run_system_with(
                            *system,
                            UiEvent {
                                receiver: sender,
                                event: event.clone(),
                                sender,
                            },
                        );
                    }
                }
                EventReceiverKind::Trigger(receiver) => {
                    commands.trigger(UiEvent {
                        receiver: *receiver,
                        event: event.clone(),
                        sender,
                    });
                }
                EventReceiverKind::Trait(receiver) => {
                    trait_entitys.push(*receiver);
                }
                EventReceiverKind::Lambda(f) => {
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
                commands.queue(move |world: &mut World| {
                    let mut system_state =
                        SystemState::<(Query<All<&dyn EventReceiver<E>>>, Commands)>::new(world);
                    let (query, mut commands) = system_state.get(world);
                    for trait_impls in trait_entitys.into_iter().filter_map(|e| query.get(e).ok()) {
                        let mut entity_commands = commands.entity(sender);
                        for impl_component in trait_impls {
                            impl_component.on_event(entity_commands.reborrow(), event.clone());
                        }
                    }

                    system_state.apply(world);
                });
            }
        }

        if self.run_sender_trigger {
            commands.trigger(UiEvent {
                receiver: sender,
                event: event.clone(),
                sender,
            });
        }

        if self.run_global_triggers {
            commands.trigger(UiEvent {
                receiver: Entity::PLACEHOLDER,
                event: event.clone(),
                sender,
            });
        }
    }

    pub fn try_send(this: Option<&Self>, event: E, sender: Entity, commands: &mut Commands) {
        if let Some(this) = this {
            this.send(event, commands);
        } else {
            let this = Self {
                this_entity: sender,
                ..Default::default()
            };
            this.send(event, commands);
        }
    }
}

pub struct SendEventCommand<E: Send + Sync + 'static + Clone> {
    pub event: E,
    pub entity: Entity,
}

impl<E: Send + Sync + 'static + Clone> Command for SendEventCommand<E> {
    fn apply(self, world: &mut World) {
        let mut system_state = SystemState::<(Query<&EventDispatcher<E>>, Commands)>::new(world);
        let (query, mut commands) = system_state.get(world);
        if let Ok(event_dispatcher) = query.get(self.entity) {
            event_dispatcher.send(self.event, &mut commands);
        }
        system_state.apply(world);
    }
}

impl<E: Send + Sync + 'static + Clone> SendEventCommand<E> {
    pub fn new(event: E, entity: Entity) -> Self {
        Self { event, entity }
    }
}

#[derive(Clone, Debug)]
pub struct Action<E: Send + Sync + 'static + Clone, S: Send + Sync + 'static + Clone> {
    phantom: PhantomData<(E, S)>,
}

impl<E: Send + Sync + 'static + Clone, S: Send + Sync + 'static + Clone> EntityCommand
    for Action<E, S>
where
    S: IntoSystem<UiEvent<E>, (), ()> + 'static,
{
    fn apply(self, entity: EntityWorldMut) {
        let entity_id = entity.id();
        let world = entity.into_world_mut();
        let systemid = world
            .get_resource::<CallbackTypeRegister>()
            .unwrap()
            .get_system::<S, UiEvent<E>, ()>();
        if let Some(mut dispatcher) = world.get_mut::<EventDispatcher<E>>(entity_id) {
            dispatcher.add_system(entity_id, systemid);
        } else {
            error!("EventDispatcher not found for entity: {:?}", entity_id);
        }
    }
}

impl<E: Send + Sync + 'static + Clone, S: Send + Sync + 'static + Clone> Component for Action<E, S>
where
    S: IntoSystem<UiEvent<E>, (), ()> + 'static,
{
    type Mutability = Mutable;

    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn on_insert() -> Option<ComponentHook> {
        Some(|mut world, context: HookContext| {
            let entity = context.entity;
            let systemid = world
                .get_resource::<CallbackTypeRegister>()
                .unwrap()
                .get_system::<S, UiEvent<E>, ()>();
            if let Some(mut dispatcher) = world.get_mut::<EventDispatcher<E>>(entity) {
                dispatcher.add_system(entity, systemid);
            } else {
                error!("EventDispatcher not found for entity: {:?}", entity);
            }
        })
    }
}

impl<E: Send + Sync + 'static + Clone, S: Send + Sync + 'static + Clone> Action<E, S> {
    pub fn new(_function: S) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

pub trait CallbackRegisterAppExt {
    fn register_callback<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: SystemInput + 'static;
}
impl CallbackRegisterAppExt for App {
    fn register_callback<F, I, M>(&mut self, system: F) -> &mut App
    where
        F: IntoSystem<I, (), M> + 'static,
        I: SystemInput + 'static,
    {
        let type_id = system.type_id();
        let system_id = self.world_mut().register_system(system);
        let mut theme = self.world_mut().resource_mut::<CallbackTypeRegister>();
        theme.systems.insert(type_id, Box::new(system_id));
        self
    }
}

#[derive(Resource, Default)]
pub struct CallbackTypeRegister {
    pub systems: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    pub triggers: HashMap<TypeId, Entity>,
}

impl CallbackTypeRegister {
    pub fn register_system<F, I, M>(&mut self, system: F, commands: &mut Commands) -> SystemId<I>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: SystemInput + Send + 'static,
    {
        let type_id = system.type_id();
        match self.systems.entry(type_id) {
            Entry::Occupied(o) => *o.get().downcast_ref().unwrap(),
            Entry::Vacant(v) => {
                let system_id = commands.register_system(system);
                v.insert(Box::new(system_id));
                system_id
            }
        }
    }

    pub fn add_to_observer<F, E: Event, B: Bundle, M>(
        &mut self,
        system: F,
        commands: &mut Commands,
        entity: Entity,
    ) where
        F: IntoObserverSystem<E, B, M>,
    {
        let type_id = system.type_id();
        match self.triggers.entry(type_id) {
            Entry::Occupied(_o) => {
                commands.queue(move |world: &mut World| {
                    if let Some(mut observer) = world.get_mut::<Observer>(entity) {
                        observer.watch_entity(entity);
                    };
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

    pub fn get_system<F, I, M>(&self) -> SystemId<I, ()>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: SystemInput + 'static,
    {
        let Some(callback) = self.systems.get(&TypeId::of::<F>()) else {
            panic!(
                "system is not registered: {system}
note: add code
```
use dway_ui_framework::event::CallbackTypeRegister;
app.register_callback({system});
``` to the plugin to register the system",
                system = type_name::<F>()
            );
        };
        *callback.as_ref().downcast_ref().unwrap()
    }

    pub fn system<F, I, M>(&self, system: F) -> SystemId<I, ()>
    where
        F: IntoSystem<I, (), M> + 'static,
        I: SystemInput + 'static,
    {
        let Some(callback) = self.systems.get(&system.type_id()) else {
            panic!(
                "system is not registered: {system}
note: add code
```
use dway_ui_framework::event::CallbackTypeRegister;
app.register_callback({system});
``` to the plugin to register the system",
                system = type_name::<F>()
            );
        };
        *callback.as_ref().downcast_ref().unwrap()
    }
}

pub fn on_despawn_later_event(mut events: MessageReader<DespawnLaterEvent>, mut commands: Commands) {
    for e in events.read() {
        if !e.is_cancelled() {
            commands.entity(e.entity).despawn();
        }
    }
}

pub fn make_callback<E>(recevier: Entity, system: SystemId<UiEvent<E>>) -> EventDispatcher<E>
where
    E: Clone + Send + Sync + 'static,
{
    EventDispatcher::new_with_system(recevier, system)
}
