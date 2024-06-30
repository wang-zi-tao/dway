use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    ffi::OsString,
    marker::PhantomData,
    os::unix::net::UnixStream,
    path::Path,
    process::{self, Stdio},
    sync::Arc,
};

use anyhow::anyhow;
use bevy::{
    ecs::{
        entity::EntityHashSet,
        event::ManualEventReader,
        query::{QueryData, QueryEntityError, WorldQuery},
        system::{Command, SystemState},
    },
    tasks::IoTaskPool,
    utils::HashMap,
};
use bevy_relationship::reexport::SmallVec;
use dway_util::eventloop::{Poller, PollerGuard};
use futures::{io::BufReader, AsyncBufReadExt, FutureExt, StreamExt};
use wayland_backend::server::{ClientId, ObjectId};
use wayland_server::{DataInit, ListeningSocket, New};

use crate::{
    client::{Client, ClientData, ClientEvents},
    prelude::*,
};

#[derive(Component, Default)]
pub struct WlResourceIndex {
    pub map: HashMap<ClientId, HashMap<TypeId, HashMap<ObjectId, Entity>>>,
}

#[derive(Default)]
pub struct NonSendMark;

#[derive(Resource, Default)]
pub struct DWayServerConfig {
    pub envs: HashMap<OsString, OsString>,
    globals: Vec<Box<dyn Fn(&DisplayHandle, Entity) -> GlobalId + Send + Sync + 'static>>,
}

pub fn add_global_dispatch<T, const VERSION: u32>(app: &mut App)
where
    DWay: GlobalDispatch<T, Entity>,
    T: wayland_server::Resource + 'static,
{
    let mut config = app.world.resource_mut::<DWayServerConfig>();
    config.add_global::<T, VERSION>();
}

impl DWayServerConfig {
    pub fn add_global<T, const VERSION: u32>(&mut self)
    where
        DWay: GlobalDispatch<T, Entity>,
        T: wayland_server::Resource + 'static,
    {
        self.globals.push(Box::new(move |display, entity| {
            info!(?entity, "create global: {}@{}", type_name::<T>(), VERSION);
            display.create_global::<DWay, T, Entity>(VERSION, entity)
        }));
    }
}

#[derive(Component)]
pub struct DWayServerMark;

#[derive(Component, Deref)]
pub struct DWayServer {
    #[deref]
    pub display: PollerGuard<wayland_server::Display<DWay>>,
    pub display_number: Option<usize>,
    pub globals: Vec<GlobalId>,
    pub envs: HashMap<OsString, OsString>,
    pub socket: PollerGuard<ListeningSocket>,
}

impl DWayServer {
    pub fn new(config: &DWayServerConfig, entity: Entity, poller: &mut Poller) -> Self {
        let display = poller.add_with_callback(
            wayland_server::Display::<DWay>::new().unwrap(),
            move |world| {
                world.send_event(DispatchDisplay(entity));
            },
        );
        let socket = poller.add_with_callback(
            ListeningSocket::bind_auto("wayland", 1..33).unwrap(),
            move |world| {
                world.send_event(DispatchDisplay(entity));
            },
        );
        info!("create wayland server {:?}", &socket.socket_name().unwrap());
        let handle = display.handle();
        for func in &config.globals {
            func(&handle, entity);
        }
        Self {
            display,
            display_number: None,
            globals: Default::default(),
            envs: config.envs.clone(),
            socket,
        }
    }

    pub fn socket_name(&self) -> String {
        self.socket
            .socket_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    }

    pub fn dispatch(&mut self, entity: Entity, world: &mut World) -> Result<()> {
        while let Some(stream) = self.socket.accept()? {
            create_client(world, entity, stream, &self.display);
        }
        {
            let mut state = DWay {
                world: world as &mut World,
            };
            self.display.dispatch_clients(&mut state)?;
        }
        self.display.flush_clients()?;
        Ok(())
    }

    pub fn spawn_process_x11(&self, mut command: process::Command) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        self.do_spawn_process(command);
    }

    pub fn spawn_process(&self, mut command: process::Command) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        command.env("WAYLAND_DISPLAY", &*self.socket_name());
        self.do_spawn_process(command);
    }

    pub fn spawn_process_wayland(&self, mut command: process::Command) {
        command
            .env("WAYLAND_DISPLAY", &*self.socket_name())
            .env_remove("DISPLAY");
        self.do_spawn_process(command);
    }

    fn do_spawn_process(&self, mut command: process::Command) {
        command.envs(&self.envs);
        IoTaskPool::get()
            .spawn(async {
                let program = command.get_program().to_string_lossy();
                let program = String::from(
                    Path::new(&*program)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                );
                let mut command: async_process::Command = command.into();
                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());

                let mut subprocess = match command.spawn() {
                    Ok(subprocess) => subprocess,
                    Err(error) => {
                        error!(%error,?command,"failed to spawn process");
                        return;
                    }
                };
                let mut stdout = BufReader::new(subprocess.stdout.take().unwrap())
                    .lines()
                    .fuse();
                let mut stderr = BufReader::new(subprocess.stderr.take().unwrap())
                    .lines()
                    .fuse();

                let id = subprocess.id();
                info!("spawn process ({program}) [{id:?}] `{command:?}`");
                loop {
                    futures::select! {
                        state=subprocess.status().fuse()=>{
                            match state{
                                Ok(o) => {
                                    info!("process ({program}) [{id:?}] exited with status: {o}");
                                },
                                Err(error) => {
                                    error!(%error);
                                },
                            }
                            break;
                        }
                        line=stdout.next()=>{
                            match line {
                                Some(Ok(line))=>{
                                    tracing::event!(
                                        target:"subprocess",
                                        tracing::Level::INFO,
                                        {},
                                        "({program}) [{id:?}] | {}",
                                        line
                                    );
                                }
                                _=>{}
                            };
                        }
                        line=stderr.next()=>{
                            match line {
                                Some(Ok(line))=>{
                                    tracing::event!(
                                        target:"subprocess",
                                        tracing::Level::INFO,
                                        {},
                                        "({program}) [{id:?}] | {}",
                                        line
                                    );
                                }
                                _=>{}
                            };
                        }
                    };
                }
            })
            .detach();
    }
}

pub struct DWay {
    world: *mut World,
}

unsafe impl Sync for DWay {
}
unsafe impl Send for DWay {
}
impl std::ops::Deref for DWay {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.world()
    }
}
impl std::ops::DerefMut for DWay {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.world_mut()
    }
}

impl DWay {
    pub fn with<R>(world: &mut World, f: impl FnOnce(&mut Self) -> R) -> R {
        let mut this = Self {
            world: world as *mut _,
        };
        f(&mut this)
    }

    pub fn world(&self) -> &World {
        unsafe { self.world.as_ref().unwrap() }
    }

    pub fn world_mut(&mut self) -> &mut World {
        unsafe { self.world.as_mut().unwrap() }
    }

    pub fn set_enum<T, R>(e: WEnum<T>, mut f: impl FnMut(T) -> R) -> Option<R> {
        match e.into_result() {
            Ok(e) => Some(f(e)),
            Err(error) => {
                error!(?error, "wrone enum");
                None
            }
        }
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
        command.apply(world);
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
        command.apply(world);
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
        command.apply(world);
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
        command.apply(world);
    }

    pub fn destroy_object(&mut self, object: &impl wayland_server::Resource) {
        let entity = DWay::get_entity(object);
        debug!(?entity,resource=%wayland_server::Resource::id(object),"destroy wayland object");
        self.despawn_tree(entity);
    }

    pub fn bind<T, C, F>(
        &mut self,
        client: &wayland_server::Client,
        resource: wayland_server::New<T>,
        data_init: &mut wayland_server::DataInit<'_, Self>,
        f: F,
    ) -> Entity
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Bundle,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        let world = self.world_mut();
        let client_data = client.get_data::<ClientData>().unwrap();
        let mut entity_command = world.entity_mut(client_data.entity);
        let entity = entity_command.id();
        let object = data_init.init(resource, entity);
        debug!(?entity,client=%client_name(&client.id()),object=%wayland_server::Resource::id(&object),"bind global object");
        entity_command.insert(f(object));
        entity
    }

    pub fn bind_spawn<T, B, F>(
        &mut self,
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
        debug!(?entity,parent=?client_data.entity,client=%client_name(&client.id()),object=%wayland_server::Resource::id(&object),"bind global object");
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
        self.spawn((resource, data_init, f).with_parent(parent))
            .id()
    }

    pub fn insert_object<T, C, F>(
        &mut self,
        entity: Entity,
        resource: New<T>,
        data_init: &mut DataInit<'_, Self>,
        f: F,
    ) -> Option<Entity>
    where
        DWay: wayland_server::Dispatch<T, Entity>,
        C: bevy::prelude::Component,
        T: wayland_server::Resource + 'static,
        F: FnOnce(T) -> C,
    {
        self.insert(
            entity,
            (resource, data_init, f).check_component_not_exists::<C>(),
        )
        .map(|e| e.id())
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
        self.spawn((resource, data_init, f).with_parent(parent))
            .id()
    }

    pub fn spawn_child<B>(&mut self, parent: Entity, bundle: B) -> Entity
    where
        B: Bundle,
    {
        self.spawn(bundle.with_parent(parent)).id()
    }

    pub fn get_entity(object: &impl wayland_server::Resource) -> Entity {
        *object
            .data::<Entity>()
            .expect("the user data type on wayland object should be Entity")
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
        if let Some(entity_mut) = self.get_entity_mut(entity) {
            trace!(?entity, "despawn recursive");
            entity_mut.despawn_recursive();
        }
    }

    pub fn despawn(&mut self, entity: Entity) {
        if let Some(e) = self.world_mut().get_entity_mut(entity) {
            if let Some(parent) = e.get::<Parent>() {
                let parent = parent.get();
                if let Some(children) = e.get::<Children>() {
                    let children = children.iter().cloned().collect::<SmallVec<[Entity; 7]>>();
                    let mut parent_entity = self.world_mut().get_entity_mut(parent).unwrap();
                    parent_entity.remove_children(&[entity]);
                    for child in children.iter() {
                        parent_entity.add_child(*child);
                    }
                }
            }
        }
        if let Some(e) = self.world_mut().get_entity_mut(entity) {
            trace!(?entity, "despawn entity");
            EntityWorldMut::despawn(e)
        }
    }

    pub fn despawn_object_component<T: Bundle>(
        &mut self,
        entity: Entity,
        resource: &impl wayland_server::Resource,
    ) {
        trace!(entity=?entity,resource=%resource.id(),"remove object component: {}",type_name::<T>());
        if let Some(mut e) = self.world_mut().get_entity_mut(entity) {
            e.remove::<T>();
        }
    }

    pub fn despawn_object(&mut self, entity: Entity, resource: &impl wayland_server::Resource) {
        trace!(entity=?entity,resource=%resource.id(),"despawn object");
        self.despawn_tree(entity);
    }

    pub fn with_component<T, F, R>(
        &mut self,
        object: &impl wayland_server::Resource,
        f: F,
    ) -> Option<R>
    where
        T: Component,
        F: FnOnce(&mut T) -> R,
    {
        let world = self.world_mut();
        let entity = Self::get_entity(object);
        let mut component = world
            .get_mut::<T>(entity)
            .ok_or_else(|| {
                anyhow!(
                    "failed to query component {} of entity {:?} ({})",
                    type_name::<T>(),
                    entity,
                    object.id()
                )
            })
            .ok()?;
        Some(f(&mut component))
    }

    pub fn query<B, F, R>(&mut self, entity: Entity, f: F) -> R
    where
        B: QueryData,
        F: FnOnce(<B as WorldQuery>::Item<'_>) -> R,
    {
        let world = self.world_mut();
        let mut query = world.query::<B>();
        f(query.get_mut(world, entity).unwrap())
    }

    pub fn try_query<B, F, R>(&mut self, entity: Entity, f: F) -> Result<R, QueryEntityError>
    where
        B: QueryData,
        F: FnOnce(<B as WorldQuery>::Item<'_>) -> R,
    {
        let world = self.world_mut();
        let mut query = world.query::<B>();
        query.get_mut(world, entity).map(f)
    }

    pub fn query_object_component<C, F, R>(
        &mut self,
        object: &impl wayland_server::Resource,
        f: F,
    ) -> R
    where
        C: Component,
        F: FnOnce(&mut C) -> R,
    {
        let world = self.world_mut();
        let entity = Self::get_entity(object);
        f(&mut world.get_mut(entity).unwrap())
    }

    pub fn query_object<B, F, R>(
        &mut self,
        object: &impl wayland_server::Resource,
        f: F,
    ) -> Option<R>
    where
        B: QueryData,
        F: FnOnce(<B as WorldQuery>::Item<'_>) -> R,
    {
        let world = self.world_mut();
        let entity = Self::get_entity(object);
        let mut query = world.query::<B>();
        Some(f(query.get_mut(world, entity).ok()?))
    }

    pub fn send_event<T: Event>(&mut self, event: T) {
        let world = self.world_mut();
        world.send_event(event);
    }
}

pub fn create_client(
    world: &mut World,
    display_entity: Entity,
    client_stream: UnixStream,
    display: &wayland_server::Display<DWay>,
) {
    let guard = unsafe {
        let mut poller = world.non_send_resource_mut::<Poller>();
        poller.add_raw_with_callback(&client_stream, move |world| {
            world.send_event(DispatchDisplay(display_entity));
        })
    };
    let entity = world.spawn_empty().id();
    let events = world.resource::<ClientEvents>().clone();
    let result = display.handle().insert_client(
        client_stream,
        Arc::new(ClientData::new(entity, &events, guard)),
    );
    let mut entity_mut = world.entity_mut(entity);
    match result {
        Ok(c) => {
            entity_mut.insert((Name::new(Cow::from(client_name(&c.id()))), Client::new(&c)));
            info!(entity=?entity_mut.id(),"add client");
        }
        Err(err) => {
            error!("Error adding wayland client: {}", err);
        }
    }
    entity_mut.set_parent(display_entity);
    let entity = entity_mut.id();
    world.send_event(Insert::<Client>::new(entity));
}

#[derive(Event)]
pub struct CreateDisplay;

#[derive(Event)]
pub struct WaylandDisplayCreated(pub Entity, pub DisplayHandle);

#[derive(Event)]
pub struct WaylandDisplayDestroyed(pub Entity, pub DisplayHandle);

pub struct DWayStatePlugin;
impl Plugin for DWayStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<NonSendMark>();
        app.init_resource::<DWayServerConfig>();
        app.add_event::<CreateDisplay>();
        app.add_event::<WaylandDisplayCreated>();
        app.add_event::<WaylandDisplayDestroyed>();
        app.add_systems(
            PreUpdate,
            (
                on_create_display_event.run_if(on_event::<CreateDisplay>()),
                apply_deferred,
            )
                .chain(),
        );
        app.add_systems(
            PreUpdate,
            (dispatch_events, apply_deferred)
                .run_if(on_event::<DispatchDisplay>())
                .in_set(DWayServerSet::Dispatch),
        );
        app.add_systems(Last, flush_display.in_set(DWayServerSet::Clean));
    }
}
pub fn on_create_display_event(
    mut events: EventReader<CreateDisplay>,
    mut commands: Commands,
    mut event_sender: EventWriter<WaylandDisplayCreated>,
    config: Res<DWayServerConfig>,
    mut poller: NonSendMut<Poller>,
) {
    for _event in events.read() {
        create_display(&mut commands, &config, &mut event_sender, &mut poller);
    }
}

pub fn create_display(
    commands: &mut Commands,
    config: &DWayServerConfig,
    event_sender: &mut EventWriter<WaylandDisplayCreated>,
    poller: &mut Poller,
) -> Entity {
    let mut entity_command = commands.spawn_empty();
    let entity = entity_command.id();

    let dway = DWayServer::new(config, entity, poller);
    let name = Name::new(Cow::Owned(format!("wayland_server@{}", dway.socket_name())));
    event_sender.send(WaylandDisplayCreated(entity, dway.display.handle()));
    entity_command.insert((name, DWayServerMark, dway));
    entity
}

pub fn dispatch_events(
    world: &mut World,
    mut event_reader: Local<ManualEventReader<DispatchDisplay>>,
) {
    let displays = event_reader
        .read(world.resource())
        .map(|e| e.0)
        .collect::<EntityHashSet>();
    for display_entity in displays {
        let mut display_entity_mut = world.entity_mut(display_entity);
        let mut server = display_entity_mut.take::<DWayServer>().unwrap();
        debug!(entity=?display_entity_mut.id(), wayland = server.socket_name() , "dispatch wayland event");
        display_entity_mut.world_scope(|world| {
            if let Err(err) = server.dispatch(display_entity, world){
                error!("failed to receive wayland requests: {err}");
            };
            if let Err(err) = server.display.flush_clients(){
                error!("failed flush wayland events buffer: {err}");
            };
        });
        display_entity_mut.insert(server);
    }
}

pub fn flush_display(mut display_query: Query<&mut DWayServer>) {
    for mut display in display_query.iter_mut() {
        if let Err(e) = display.display.flush_clients(){
            error!("failed to flush wayland display: {e}");
        };
    }
}

pub fn client_name(id: &ClientId) -> String {
    let name = format!("{:?}", id)[21..35].to_string();
    format!("client@{}", name)
}

impl DWay {
    pub fn insert<T>(
        &mut self,
        entity: Entity,
        f: impl EntityFactory<T>,
    ) -> Option<EntityWorldMut> {
        let world = self.world_mut();
        f.insert(world, entity)
    }

    pub fn spawn<T>(&mut self, f: impl EntityFactory<T>) -> EntityWorldMut {
        f.spawn(self.world_mut())
    }
}

pub trait EntityFactory<T> {
    fn spawn(self, world: &mut World) -> EntityWorldMut<'_>
    where
        Self: Sized;
    fn insert(self, world: &mut World, entity: Entity) -> Option<EntityWorldMut<'_>>;

    fn with_parent(self, parent: Entity) -> WithParent<Self, T>
    where
        Self: Sized,
    {
        WithParent {
            inner: self,
            parent,
            phanton: PhantomData,
        }
    }
    fn check_component_not_exists<C>(self) -> CheckNoComponent<Self, T, C>
    where
        Self: Sized,
        C: Component,
    {
        CheckNoComponent {
            inner: self,
            phanton: PhantomData,
        }
    }
    fn connect_to<R>(self, target: Entity) -> ConnectTo<Self, T, R>
    where
        Self: Sized,
        R: Relationship + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        ConnectTo {
            inner: self,
            target,
            phanton: PhantomData,
        }
    }
    fn connect_from<R>(self, target: Entity) -> ConnectTo<Self, T, ReserveRelationship<R>>
    where
        Self: Sized,
        R: Relationship + Send + Sync + 'static,
        R::From: ConnectableMut + Default,
        R::To: ConnectableMut + Default,
    {
        ConnectTo {
            inner: self,
            target,
            phanton: PhantomData,
        }
    }
}

impl<T: Bundle> EntityFactory<(T,)> for T {
    fn spawn(self, world: &mut World) -> EntityWorldMut {
        world.spawn(self)
    }

    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let mut entity_mut = world.get_entity_mut(entity)?;
        entity_mut.insert(self);
        Some(entity_mut)
    }
}
impl<T: FnOnce() -> B, B: Bundle> EntityFactory<()> for T {
    fn spawn(self, world: &mut World) -> EntityWorldMut {
        world.spawn(self())
    }

    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let mut entity_mut = world.get_entity_mut(entity)?;
        entity_mut.insert(self());
        Some(entity_mut)
    }
}
impl<T: FnOnce(&mut World) -> B, B: Bundle> EntityFactory<(&mut World,)> for T {
    fn spawn(self, world: &mut World) -> EntityWorldMut {
        let bundle = self(world);
        world.spawn(bundle)
    }

    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let bundle = self(world);
        let mut entity_mut = world.get_entity_mut(entity)?;
        entity_mut.insert(bundle);
        Some(entity_mut)
    }
}
impl<'l, 'd, T, F, B> EntityFactory<(T,)> for (New<T>, &'l mut DataInit<'d, DWay>, F)
where
    F: FnOnce(T) -> B,
    B: Bundle,
    T: WlResource + 'static,
    DWay: wayland_server::Dispatch<T, Entity>,
{
    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let (resource, data_init, f) = self;
        let object = data_init.init(resource, entity);
        let mut entity_mut = world.get_entity_mut(entity)?;
        debug!(entity=?entity_mut.id(),object=%wayland_server::Resource::id(&object),"new wayland object");
        entity_mut.insert(f(object));
        Some(entity_mut)
    }

    fn spawn(self, world: &mut World) -> EntityWorldMut<'_>
    where
        Self: Sized,
    {
        let (resource, data_init, f) = self;
        let mut entity_mut = world.spawn_empty();
        let object = data_init.init(resource, entity_mut.id());
        debug!(entity=?entity_mut.id(),object=%wayland_server::Resource::id(&object),"new wayland object");
        entity_mut.insert((Name::new(Cow::from(object.id().to_string())), f(object)));
        entity_mut
    }
}
impl<'l, 'd, T, F, B> EntityFactory<(T, World)> for (New<T>, &'l mut DataInit<'d, DWay>, F)
where
    F: FnOnce(T, &mut World) -> B,
    B: Bundle + 'static,
    T: WlResource + 'static,
    DWay: wayland_server::Dispatch<T, Entity>,
{
    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let (resource, data_init, f) = self;
        let object = data_init.init(resource, entity);
        debug!(entity=?entity,object=%wayland_server::Resource::id(&object),"new wayland object");
        let bundle: B = f(object, world);
        let mut entity_mut = world.get_entity_mut(entity)?;
        entity_mut.insert(bundle);
        Some(entity_mut)
    }

    fn spawn(self, world: &mut World) -> EntityWorldMut<'_>
    where
        Self: Sized,
    {
        let entity = world.spawn_empty().id();
        let (resource, data_init, f) = self;
        let object = data_init.init(resource, entity);
        let name = Name::new(Cow::from(object.id().to_string()));
        debug!(entity=?entity,object=%wayland_server::Resource::id(&object),"new wayland object");
        let bundle: B = f(object, world);
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert((name, bundle));
        entity_mut
    }
}
pub struct WithParent<F: EntityFactory<T>, T> {
    pub inner: F,
    pub parent: Entity,
    pub phanton: PhantomData<T>,
}
impl<F, T> EntityFactory<T> for WithParent<F, T>
where
    F: EntityFactory<T>,
{
    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let mut entity = self.inner.insert(world, entity)?;
        entity.set_parent(self.parent);
        Some(entity)
    }

    fn spawn(self, world: &mut World) -> EntityWorldMut
    where
        Self: Sized,
    {
        let mut entity_mut = self.inner.spawn(world);
        entity_mut.set_parent(self.parent);
        entity_mut
    }
}
pub struct CheckNoComponent<F: EntityFactory<T>, T, C: Component> {
    pub inner: F,
    pub phanton: PhantomData<(T, C)>,
}
impl<F, T, C> EntityFactory<T> for CheckNoComponent<F, T, C>
where
    F: EntityFactory<T>,
    C: Component,
{
    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        assert!(
            !world.entity_mut(entity).contains::<C>(),
            "component {} already exist in entity {entity:?}",
            type_name::<C>()
        );
        self.inner.insert(world, entity)
    }

    fn spawn(self, world: &mut World) -> EntityWorldMut
    where
        Self: Sized,
    {
        self.inner.spawn(world)
    }
}
pub struct ConnectTo<F: EntityFactory<T>, T, R: Relationship> {
    pub inner: F,
    pub target: Entity,
    pub phanton: PhantomData<(T, R)>,
}

impl<F, T, R> EntityFactory<T> for ConnectTo<F, T, R>
where
    F: EntityFactory<T>,
    R: Relationship + Send + Sync + 'static,
    R::From: ConnectableMut + Default,
    R::To: ConnectableMut + Default,
{
    fn insert(
        self,
        world: &mut World,
        entity: Entity,
    ) -> Option<bevy::prelude::EntityWorldMut<'_>> {
        let target = self.target;
        let entity_mut = self.inner.insert(world, entity)?;
        let entity = entity_mut.id();
        let command = ConnectCommand::<R>::new(entity, target);
        trace!(
            "connect ({:?})-[{}]->({:?})",
            entity,
            type_name::<R>(),
            target
        );
        command.apply(world);
        world.get_entity_mut(entity)
    }

    fn spawn(self, world: &mut World) -> EntityWorldMut
    where
        Self: Sized,
    {
        let target = self.target;
        let entity_mut = self.inner.spawn(world);
        let entity = entity_mut.id();
        let command = ConnectCommand::<R>::new(entity, target);
        trace!(
            "connect ({:?})-[{}]->({:?})",
            entity,
            type_name::<R>(),
            target
        );
        command.apply(world);
        world.entity_mut(entity)
    }
}
