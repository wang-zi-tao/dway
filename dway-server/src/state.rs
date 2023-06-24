use std::{
    any::type_name,
    borrow::Cow,
    os::{fd::AsRawFd, unix::net::UnixStream},
    process,
    ptr::null_mut,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use bevy::ecs::{
    query::WorldQuery,
    system::{Command, LogComponents, Spawn},
    world::EntityMut,
};
use bevy_relationship::{
    ConnectCommand, ConnectableMut, DisconnectAllCommand, DisconnectCommand, Relationship,
    ReverseRelationship,
};
use calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction};
use inlinable_string::InlinableString;
use send_wrapper::SendWrapper;
use wayland_backend::server::ClientId;
use wayland_server::{DataInit, New};

use crate::{
    client::{Client, ClientData},
    eventloop::ListeningSocketEvent,
    prelude::*,
    schedule::DWayServerSet,
};

#[derive(Default)]
pub struct NonSendMark;

#[derive(Reflect, Resource, Default)]
pub struct DWayDisplayIndex {}

#[derive(Component, Clone)]
pub struct DWayDisplay(pub Arc<Mutex<wayland_server::Display<DWay>>>);

#[derive(Component, Clone)]
pub struct DWayWrapper(pub Arc<Mutex<DWay>>);

#[derive(Component, Clone)]
pub struct DWayEventLoop(pub Arc<Mutex<SendWrapper<EventLoop<'static, DWay>>>>);

#[derive(Bundle)]
pub struct DWayDisplayBundle {
    name: Name,
    dway: DWayWrapper,
    display: DWayDisplay,
    event_loop: DWayEventLoop,
}
pub struct DWay {
    pub(crate) world: *mut World,
    pub socket_name: InlinableString,
    pub display_number: Option<usize>,
    pub globals: Vec<GlobalId>,
}
unsafe impl Sync for DWay {}
unsafe impl Send for DWay {}

impl DWay {
    pub fn world(&self) -> &World {
        unsafe { self.world.as_ref().unwrap() }
    }
    pub fn world_mut(&mut self) -> &mut World {
        unsafe { self.world.as_mut().unwrap() }
    }
    pub fn add_global<T>(&mut self, version: u32, entity: Entity, display_handle: &DisplayHandle)
    where
        Self: GlobalDispatch<T, Entity>,
        T: wayland_server::Resource + 'static,
    {
        let global_id = display_handle.create_global::<Self, T, Entity>(version, entity);
        info!(?entity, "create global: {}", type_name::<T>());
        self.globals.push(global_id);
    }
    pub fn spawn(&self, mut command: process::Command) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        command
            .env("WAYLAND_DISPLAY", &*self.socket_name)
            .spawn()
            .unwrap();
    }
    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        let world = self.world_mut();
        world.entity_mut(parent).add_child(child);
    }
    pub fn connect<R>(&mut self, from: Entity, to: Entity)
    where
        R: Relationship + Default + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let world = self.world_mut();
        let command = ConnectCommand::<R>::new(from, to);
        trace!("connect ({:?})-[{}]->({:?})", from, type_name::<R>(), to);
        command.write(world);
    }
    pub fn disconnect<R>(&mut self, from: Entity, to: Entity)
    where
        R: Relationship + Default + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let world = self.world_mut();
        let command = DisconnectCommand::<R>::new(from, to);
        trace!(
            "disconnect ({:?})-X-[{}]-X->({:?})",
            from,
            type_name::<R>(),
            to
        );
        command.write(world);
    }
    pub fn disconnect_all<R>(&mut self, from: Entity)
    where
        R: Relationship + Default + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let world = self.world_mut();
        let command = DisconnectAllCommand::<R>::new(from);
        trace!(
            "disconnect all ({:?})-X-[{}]-X->(*)",
            from,
            type_name::<R>(),
        );
        command.write(world);
    }
    pub fn disconnect_all_rev<R>(&mut self, from: Entity)
    where
        R: Relationship + Default + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        let world = self.world_mut();
        let command = DisconnectAllCommand::<ReverseRelationship<R>>::new(from);
        trace!(
            "disconnect all (*)-X-[{}]-X->({:?})",
            type_name::<R>(),
            from,
        );
        command.write(world);
    }
    pub fn spawn_wayland(&self, mut command: process::Command) {
        command
            .env("WAYLAND_DISPLAY", &*self.socket_name)
            .env_remove("DISPLAY")
            .spawn()
            .unwrap();
    }
    pub fn destroy_object<C>(&mut self, object: &impl wayland_server::Resource) {
        let entity = DWay::get_entity(object);
        let world = self.world_mut();
        trace!(?entity,resource=%wayland_server::Resource::id(object),"destroy wayland object");
        world.entity_mut(entity).despawn_recursive();
    }
    pub fn create_client(
        &mut self,
        parent: Entity,
        client_stream: UnixStream,
        display: &wayland_server::Display<DWay>,
    ) {
        let world = self.world_mut();
        let mut entity = world.spawn_empty();
        match display
            .handle()
            .insert_client(client_stream, Arc::new(ClientData::new(entity.id())))
        {
            Ok(c) => {
                entity.insert((Name::new(Cow::from(client_name(&c.id()))), Client::new(c)));
                entity.set_parent(parent);
                trace!(entity=?entity.id(),"add client");
            }
            Err(err) => {
                error!("Error adding wayland client: {}", err);
            }
        }
    }
    pub fn bind<T, C, F>(
        self: &mut Self,
        client: &wayland_server::Client,
        resource: wayland_server::New<T>,
        data_init: &mut wayland_server::DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        let world = self.world_mut();
        let client_data = client.get_data::<ClientData>().unwrap();
        let mut entity_command = world.entity_mut(client_data.entity);
        let entity = entity_command.id();
        let object = data_init.init(resource, entity);
        trace!(?entity,client=?client_name(&client.id()),object=%wayland_server::Resource::id(&object),"bind global object");
        entity_command.insert(f(object));
        entity
    }
    pub fn bind_spawn<T, B, F>(
        self: &mut Self,
        client: &wayland_server::Client,
        resource: wayland_server::New<T>,
        data_init: &mut wayland_server::DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        B: bevy::prelude::Bundle,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> B,
    {
        let world = self.world_mut();
        let client_data = client.get_data::<ClientData>().unwrap();
        let mut entity_command = world.spawn_empty();
        let entity = entity_command.id();
        let object = data_init.init(resource, entity);
        trace!(?entity,parent=?client_data.entity,client=%client_name(&client.id()),object=%wayland_server::Resource::id(&object),"bind global object");
        entity_command.insert((Name::new(Cow::from(object.id().to_string())), f(object)));
        world.entity_mut(client_data.entity).add_child(entity);
        entity
    }
    pub fn spawn_child_object_bundle<B, T, F>(
        &mut self,
        parent: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        T: wayland_server::Resource + 'static,
        B: Bundle,
        F: FnOnce(T) -> B,
    {
        let world = self.world_mut();
        let mut entity_mut = world.spawn_empty();
        let object = data_init.init(resource, entity_mut.id());
        trace!(entity=?entity_mut.id(),object=%wayland_server::Resource::id(&object),"insert object");
        entity_mut.insert((Name::new(Cow::from(object.id().to_string())), f(object)));
        let entity = entity_mut.id();
        world.entity_mut(parent).add_child(entity);
        entity
    }
    pub fn insert_object_bundle<C, T, B, F>(
        &mut self,
        entity: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        B: bevy::prelude::Bundle,
        F: FnOnce(T) -> B,
    {
        let world = self.world_mut();
        assert!(
            !world.entity_mut(entity).contains::<C>(),
            "component {} already exist in entity {entity:?}",
            type_name::<C>()
        );
        let object = data_init.init(resource, entity);
        trace!(?entity,object=%wayland_server::Resource::id(&object),"insert object");
        world.entity_mut(entity).insert(f(object));
        entity
    }
    pub fn insert_object<T, C, F>(
        &mut self,
        entity: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        let world = self.world_mut();
        assert!(
            !world.entity_mut(entity).contains::<C>(),
            "component {} already exist in entity {entity:?}",
            type_name::<C>()
        );
        let object = data_init.init(resource, entity);
        trace!(?entity,object=%wayland_server::Resource::id(&object),"insert object");
        world.entity_mut(entity).insert(f(object));
        entity
    }
    pub fn spawn_child_object<T, C, F>(
        &mut self,
        parent: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        let world = self.world_mut();
        let mut entity_command = world.spawn_empty();
        let entity = entity_command.id();
        let object = data_init.init(resource, entity);
        trace!(parent=?parent,?entity,object=%wayland_server::Resource::id(&object),"spawn object");
        entity_command.insert((Name::new(Cow::from(object.id().to_string())), f(object)));
        world.entity_mut(parent).add_child(entity);
        entity
    }
    pub fn insert_child(&mut self, parent: Entity, entity: Entity, b: impl Bundle) -> Entity
where {
        let world = self.world_mut();
        let mut entity_command = world.entity_mut(entity);
        trace!(parent=?parent,?entity,"move object");
        entity_command.insert(b);
        world.entity_mut(parent).add_child(entity);
        entity
    }
    pub fn insert_child_object<T, C, F>(
        &mut self,
        parent: Entity,
        entity: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        let world = self.world_mut();
        let mut entity_command = world.entity_mut(entity);
        let object = data_init.init(resource, entity);
        trace!(parent=?parent,?entity,object=%wayland_server::Resource::id(&object),"move object");
        entity_command.insert(f(object));
        world.entity_mut(parent).add_child(entity);
        entity
    }
    pub fn get_entity(object: &impl wayland_server::Resource) -> Entity {
        *object.data::<Entity>().unwrap()
    }
    pub fn client_entity(client: &wayland_server::Client) -> Entity {
        client
            .get_data::<crate::client::ClientData>()
            .unwrap()
            .entity
    }
    pub fn component<T: Component>(&self, entity: Entity) -> &T {
        self.world().entity(entity).get::<T>().unwrap()
    }
    pub fn object_component<T: Component>(&self, object: &impl wayland_server::Resource) -> &T {
        self.world()
            .entity(DWay::get_entity(object))
            .get::<T>()
            .unwrap()
    }
    pub fn despawn_tree(&mut self, entity: Entity) {
        self.world_mut().entity_mut(entity).despawn_recursive();
    }
    pub fn despawn(&mut self, entity: Entity) {
        self.world_mut()
            .get_entity_mut(entity)
            .map(EntityMut::despawn);
    }
    pub fn despawn_object(&mut self, entity: Entity, id: wayland_backend::server::ObjectId) {
        trace!(entity=?entity,resource=%id,"despawn object");
        self.world_mut()
            .get_entity_mut(entity)
            .map(EntityMut::despawn);
    }
    pub fn with_component<T, F, R>(&mut self, object: &impl wayland_server::Resource, f: F) -> R
    where
        T: Component,
        F: FnOnce(&mut T) -> R,
    {
        let world = self.world_mut();
        let entity = Self::get_entity(object);
        let mut entity_mut = world.entity_mut(entity);
        let mut component = entity_mut.get_mut::<T>().unwrap();
        f(&mut component)
    }
    pub fn query<B, F, R>(&mut self, entity: Entity, f: F) -> R
    where
        B: WorldQuery,
        F: FnOnce(<B as WorldQuery>::Item<'_>) -> R,
    {
        let world = self.world_mut();
        let mut query = world.query::<B>();
        f(query.get_mut(world, entity).unwrap())
    }
    pub fn query_object<B, F, R>(&mut self, object: &impl wayland_server::Resource, f: F) -> R
    where
        B: WorldQuery,
        F: FnOnce(<B as WorldQuery>::Item<'_>) -> R,
    {
        let world = self.world_mut();
        let entity = Self::get_entity(object);
        let mut query = world.query::<B>();
        f(query.get_mut(world, entity).unwrap())
    }
    pub fn send_event<T: Event>(&mut self, event: T) {
        let world = self.world_mut();
        world.send_event(event);
    }
}

pub struct CreateDisplay;
pub struct DisplayCreated(pub Entity, pub DisplayHandle);
pub struct DWayStatePlugin;
impl Plugin for DWayStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<NonSendMark>();
        app.add_event::<CreateDisplay>();
        app.add_event::<DisplayCreated>();
        app.add_systems(
            (
                on_create_display_event.run_if(on_event::<CreateDisplay>()),
                apply_system_buffers,
            )
                .chain()
                .in_set(DWayServerSet::Create),
        );
        app.add_system(dispatch_events.in_set(DWayServerSet::Dispatch));
    }
}
pub fn on_create_display_event(
    _: NonSend<NonSendMark>,
    mut events: EventReader<CreateDisplay>,
    mut commands: Commands,
    mut event_sender: EventWriter<DisplayCreated>,
) {
    for event in events.iter() {
        create_display(&mut commands, &mut event_sender);
    }
}

pub fn create_display(
    mut commands: &mut Commands,
    mut event_sender: &mut EventWriter<DisplayCreated>,
) -> Entity {
    let mut entity_command = commands.spawn_empty();
    let entity = entity_command.id();

    let mut event_loop = EventLoop::try_new().unwrap();
    let mut display = wayland_server::Display::<DWay>::new().unwrap();

    let handle: DisplayHandle = display.handle();
    let source = ListeningSocketEvent::new();
    let socket_name = source.filename();

    info!("listening on {}", &socket_name);
    event_loop
        .handle()
        .insert_source(source, move |client_stream, _, data: &mut DWay| {
            let display = data.component::<DWayDisplay>(entity).0.clone();
            data.create_client(entity, client_stream, &display.lock().unwrap());
        })
        .expect("Failed to init wayland socket source");
    event_loop
        .handle()
        .insert_source(
            Generic::new(
                display.backend().poll_fd().as_raw_fd(),
                Interest::READ,
                Mode::Level,
            ),
            move |_, _, state| {
                let display = state.component::<DWayDisplay>(entity).0.clone();
                display.lock().unwrap().dispatch_clients(state).unwrap();
                Ok(PostAction::Continue)
            },
        )
        .unwrap();

    let name = Name::new(Cow::Owned(format!("wayland_server@{socket_name}")));
    let state = DWay {
        world: null_mut(),
        socket_name,
        display_number: None,
        globals: Vec::new(),
    };
    entity_command.insert(DWayDisplayBundle {
        dway: DWayWrapper(Arc::new(Mutex::new(state))),
        display: DWayDisplay(Arc::new(Mutex::new(display))),
        event_loop: DWayEventLoop(Arc::new(Mutex::new(SendWrapper::new(event_loop)))),
        name,
    });
    event_sender.send(DisplayCreated(entity, handle));
    entity
}
pub fn dispatch_events(world: &mut World) {
    let mut display_query: QueryState<(&DWayWrapper, &DWayDisplay, &DWayEventLoop)> = world.query();
    let displays: Vec<_> = display_query
        .iter(world)
        .map(|(w, d, e)| (w.clone(), d.clone(), e.clone()))
        .collect();
    let start_time = Instant::now();
    let duration = Duration::from_secs_f32(0.004);
    let end_time = start_time + duration;
    for (dway, display, events) in &displays {
        let mut dway = dway.0.lock().unwrap();
        dway.world = world as *mut World;
        let mut event_loop = events.0.lock().unwrap();
        event_loop.dispatch(Some(duration), &mut dway).unwrap();
        dway.world = null_mut();
    }
    loop {
        let now = Instant::now();
        if now > end_time {
            break;
        }
        for (dway, display, events) in &displays {
            let mut dway = dway.0.lock().unwrap();
            dway.world = world as *mut World;
            let mut display = display.0.lock().unwrap();
            display.dispatch_clients(&mut dway).unwrap();
            display.flush_clients().unwrap();
            dway.world = null_mut();
        }
    }
}

pub fn create_global<T, const VERSION: u32>(
    mut events: EventReader<DisplayCreated>,
    dway_query: Query<&DWayWrapper>,
) where
    DWay: GlobalDispatch<T, Entity>,
    T: wayland_server::Resource + 'static,
{
    for event in events.iter() {
        if let Ok(dway) = dway_query.get(event.0) {
            dway.0
                .lock()
                .unwrap()
                .add_global(VERSION, event.0, &event.1);
        }
    }
}
pub fn create_global_system_config<T, const VERSION: u32>() -> bevy::ecs::schedule::SystemConfig
where
    DWay: GlobalDispatch<T, Entity>,
    T: wayland_server::Resource + 'static,
{
    create_global::<T, VERSION>
        .in_set(DWayServerSet::CreateGlobal)
        .run_if(on_event::<DisplayCreated>())
}
pub fn client_name(id: &ClientId) -> String {
    let name = format!("{:?}", id)[21..35].to_string();
    format!("client@{}", name)
}
