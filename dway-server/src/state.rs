use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    ffi::OsString,
    io::BufRead,
    marker::PhantomData,
    os::{fd::AsRawFd, unix::net::UnixStream},
    path::Path,
    process::{self, Stdio},
    ptr::null_mut,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::anyhow;
use bevy::{
    ecs::{
        query::{QueryEntityError, WorldQuery},
        system::Command,
        world::EntityMut,
    },
    utils::{tracing, HashMap},
};
use bevy_relationship::{
    reexport::SmallVec, ConnectCommand, ConnectableMut, DisconnectAllCommand, DisconnectCommand,
    Relationship, ReserveRelationship, ReverseRelationship,
};
use bevy_tokio_tasks::TokioTasksRuntime;
use calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction};
use dway_winit::{FrameConditionSchedule, UpdateRequest, UpdateRequestEvents};
use inlinable_string::InlinableString;
use send_wrapper::SendWrapper;
use tokio::io::AsyncReadExt;
use wayland_backend::server::{ClientId, ObjectId};
use wayland_server::{DataInit, New};

use crate::{
    client::{Client, ClientData},
    eventloop::ListeningSocketEvent,
    prelude::*,
    schedule::DWayServerSet,
};

#[derive(Component, Default)]
pub struct WlResourceIndex {
    pub map: HashMap<ClientId, HashMap<TypeId, HashMap<ObjectId, Entity>>>,
}

#[derive(Default)]
pub struct NonSendMark;

#[derive(Reflect, Resource, Default)]
pub struct DWayDisplayIndex {}

#[derive(Component, Clone)]
pub struct DWayDisplay(pub Arc<Mutex<wayland_server::Display<DWay>>>);

#[derive(Component, Clone)]
pub struct DWayWrapper(pub Arc<Mutex<DWay>>);

impl lazy_static::__Deref for DWayWrapper {
    type Target = Arc<Mutex<DWay>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Clone)]
pub struct DWayEventLoop(pub Arc<Mutex<SendWrapper<EventLoop<'static, DWay>>>>);

#[derive(Bundle)]
pub struct DWayDisplayBundle {
    name: Name,
    dway: DWayWrapper,
    display: DWayDisplay,
    event_loop: DWayEventLoop,
}

impl DWayDisplayBundle {
    pub fn new(
        name: Name,
        dway: DWayWrapper,
        display: DWayDisplay,
        event_loop: DWayEventLoop,
    ) -> Self {
        Self {
            name,
            dway,
            display,
            event_loop,
        }
    }
}
pub struct DWay {
    world: *mut World,
    pub display_handle: DisplayHandle,
    pub socket_name: InlinableString,
    pub display_number: Option<usize>,
    pub globals: Vec<GlobalId>,
    pub envs: HashMap<OsString, OsString>,
}
unsafe impl Sync for DWay {}
unsafe impl Send for DWay {}

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
    pub fn set_enum<T, R>(e: WEnum<T>, mut f: impl FnMut(T) -> R) -> Option<R> {
        match e.into_result() {
            Ok(e) => Some(f(e)),
            Err(error) => {
                error!(?error, "wrone enum");
                None
            }
        }
    }
    pub fn spawn_process_x11(&self, mut command: process::Command, tokio: &TokioTasksRuntime) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        self.do_spawn_process(command, tokio);
    }
    pub fn spawn_process(&self, mut command: process::Command, tokio: &TokioTasksRuntime) {
        if let Some(display_number) = self.display_number {
            command.env("DISPLAY", ":".to_string() + &display_number.to_string());
        } else {
            command.env_remove("DISPLAY");
        }
        command.env("WAYLAND_DISPLAY", &*self.socket_name);
        self.do_spawn_process(command, tokio);
    }
    pub fn spawn_process_wayland(&self, mut command: process::Command, tokio: &TokioTasksRuntime) {
        command
            .env("WAYLAND_DISPLAY", &*self.socket_name)
            .env_remove("DISPLAY");
        self.do_spawn_process(command, tokio);
    }
    fn do_spawn_process(&self, mut command: process::Command, tokio: &TokioTasksRuntime) {
        command.envs(&self.envs);
        tokio.spawn_background_task(|_ctx| async move {
            let program = command.get_program().to_string_lossy();
            let program = String::from(
                Path::new(&*program)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
            );
            let mut command: tokio::process::Command = command.into();
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());

            let mut subprocess = match command.spawn() {
                Ok(subprocess) => subprocess,
                Err(error) => {
                    error!(%error,?command,"failed to spawn process");
                    return;
                }
            };
            let mut stdout = subprocess.stdout.take().unwrap();
            let mut stderr = subprocess.stderr.take().unwrap();

            let id = subprocess.id().unwrap_or_default();
            info!("process ({program}) [{id:?}] spawn");
            let mut stdout_buffer = [0; 256];
            let mut stderr_buffer = [0; 256];
            let print_output = |buffer: &[u8; 256], result| {
                let size = match result {
                    Ok(size) => size,
                    Err(e) => {
                        error!("process ({program}) [{id:?}] io error: {e}");
                        return;
                    }
                };
                if size == 0 {
                    return;
                }
                let buffer = &buffer[..size];
                for line in buffer.lines() {
                    if let Ok(line) = line {
                        tracing::event!(
                            target:"subprocess",
                            tracing::Level::INFO,
                            {},
                            "({program}) [{id:?}] | {}",
                            line
                        );
                    }
                }
            };
            loop {
                tokio::select! {
                    o=subprocess.wait()=>{
                        match o{
                            Ok(o) => {
                                info!("process ({program}) [{id:?}] exited with status: {o}");
                            },
                            Err(error) => {
                                error!(%error);
                            },
                        }
                        return;
                    }
                    size=stdout.read(&mut stdout_buffer)=>{
                        print_output(&stdout_buffer,size);
                    }
                    size=stderr.read(&mut stderr_buffer)=>{
                        print_output(&stdout_buffer,size);
                    }
                };
            }
        });
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
    pub fn destroy_object(&mut self, object: &impl wayland_server::Resource) {
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
                info!(entity=?entity.id(),"add client");
            }
            Err(err) => {
                error!("Error adding wayland client: {}", err);
            }
        }
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
    ) -> Entity
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
        .id()
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
        if let Some(e) = self.world_mut().get_entity_mut(entity) {
            if let Some(parent) = e.get::<Parent>() {
                let parent = parent.get();
                if let Some(children) = e.get::<Children>() {
                    let children = children.iter().collect::<SmallVec<[Entity; 7]>>();
                    let mut parent_entity = self.world_mut().get_entity_mut(parent).unwrap();
                    parent_entity.remove_children(&[entity]);
                    for child in children.iter() {
                        parent_entity.add_child(*child);
                    }
                }
            }
        }
        if let Some(e) = self.world_mut().get_entity_mut(entity) {
            EntityMut::despawn(e)
        }
    }
    pub fn despawn_object(&mut self, entity: Entity, id: wayland_backend::server::ObjectId) {
        trace!(entity=?entity,resource=%id,"despawn object");
        if let Some(mut e) = self.world_mut().get_entity_mut(entity) {
            e.despawn_recursive();
        }
    }
    pub fn with_component<T, F, R>(&mut self, object: &impl wayland_server::Resource, f: F) -> R
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
            .unwrap();
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
    pub fn try_query<B, F, R>(&mut self, entity: Entity, f: F) -> Result<R, QueryEntityError>
    where
        B: WorldQuery,
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
    pub fn scope<R>(&mut self, world: &mut World, f: impl FnOnce(&mut Self) -> R) -> R {
        assert!(self.world.is_null());
        self.world = world as *mut World;
        let r = f(self);
        self.world = null_mut();
        r
    }
}

pub struct CreateDisplay;
pub struct WaylandDisplayCreated(pub Entity, pub DisplayHandle);
pub struct WaylandDisplayDestroyed(pub Entity, pub DisplayHandle);
pub struct DWayStatePlugin;
impl Plugin for DWayStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<NonSendMark>();
        app.add_event::<CreateDisplay>();
        app.add_event::<WaylandDisplayCreated>();
        app.add_event::<WaylandDisplayDestroyed>();
        app.add_systems(
            (
                on_create_display_event.run_if(on_event::<CreateDisplay>()),
                apply_system_buffers,
            )
                .chain()
                .in_set(DWayServerSet::Create),
        );
        app.add_system(frame_condition.in_schedule(FrameConditionSchedule));
        app.add_system(dispatch_events.in_set(DWayServerSet::Dispatch));
        // app.add_system(flush_display.in_set(DWayServerSet::InputFlush));
        // app.add_system(flush_display.in_set(DWayServerSet::PostUpdate));
        // app.add_system(flush_display.in_set(DWayServerSet::Last));
        set_signal_handler();
    }
}
pub fn on_create_display_event(
    _: NonSend<NonSendMark>,
    mut events: EventReader<CreateDisplay>,
    mut commands: Commands,
    mut event_sender: EventWriter<WaylandDisplayCreated>,
    mut update_request_eventss: NonSend<UpdateRequestEvents>,
) {
    for _event in events.iter() {
        create_display(
            &mut commands,
            &mut event_sender,
            &mut update_request_eventss,
        );
    }
}

pub fn create_display(
    commands: &mut Commands,
    event_sender: &mut EventWriter<WaylandDisplayCreated>,
    update_request_eventss: &mut NonSend<UpdateRequestEvents>,
) -> Entity {
    let mut entity_command = commands.spawn_empty();
    let entity = entity_command.id();

    let event_loop = EventLoop::try_new().unwrap();
    let mut display = wayland_server::Display::<DWay>::new().unwrap();

    let handle: DisplayHandle = display.handle();
    let source = ListeningSocketEvent::new();
    let socket_name = source.filename();

    let sender = update_request_eventss.sender.clone();
    let sender_clone = sender.clone();

    info!("listening on {}", &socket_name);
    event_loop
        .handle()
        .insert_source(source, move |client_stream, _, data: &mut DWay| {
            let display = data.component::<DWayDisplay>(entity).0.clone();
            data.create_client(entity, client_stream, &display.lock().unwrap());
            let _ = sender_clone.send(UpdateRequest::default());
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
                let _guard = display.lock().unwrap();
                let _ = sender.send(UpdateRequest::default());
                Ok(PostAction::Continue)
            },
        )
        .unwrap();

    let name = Name::new(Cow::Owned(format!("wayland_server@{socket_name}")));
    let state = DWay {
        world: null_mut(),
        display_handle: handle.clone(),
        socket_name,
        display_number: None,
        globals: Vec::new(),
        envs: Default::default(),
    };
    entity_command.insert(DWayDisplayBundle::new(
        name,
        DWayWrapper(Arc::new(Mutex::new(state))),
        DWayDisplay(Arc::new(Mutex::new(display))),
        DWayEventLoop(Arc::new(Mutex::new(SendWrapper::new(event_loop)))),
    ));
    event_sender.send(WaylandDisplayCreated(entity, handle));
    entity
}

pub fn frame_condition(world: &mut World) {
    let mut display_query: QueryState<(&DWayWrapper, &DWayEventLoop)> = world.query();
    let displays: Vec<_> = display_query
        .iter(world)
        .map(|(w, e)| (w.clone(), e.clone()))
        .collect();
    let duration = Duration::from_secs_f32(0.001);
    for (dway, events) in &displays {
        let mut dway = dway.0.lock().unwrap();
        dway.scope(world, |dway| {
            let mut event_loop = events.0.lock().unwrap();
            event_loop.dispatch(Some(duration), dway).unwrap();
        });
    }
}

pub fn dispatch_events(world: &mut World) {
    let mut display_query: QueryState<(&DWayWrapper, &DWayDisplay, &DWayEventLoop)> = world.query();
    let displays: Vec<_> = display_query
        .iter(world)
        .map(|(w, d, e)| (w.clone(), d.clone(), e.clone()))
        .collect();
    let duration = Duration::from_secs_f32(0.001);
    for (dway, display, events) in &displays {
        let mut dway = dway.0.lock().unwrap();
        dway.scope(world, |dway| {
            let mut event_loop = events.0.lock().unwrap();
            event_loop.dispatch(Some(duration), dway).unwrap();
            let mut display = display.0.lock().unwrap();
            display.dispatch_clients(dway).unwrap();
            display.flush_clients().unwrap();
        });
    }
}

pub fn flush_display(display_query: Query<&DWayDisplay>) {
    display_query.for_each(|display| {
        display.0.lock().unwrap().flush_clients().unwrap();
    })
}

pub fn create_global<T, const VERSION: u32>(
    mut events: EventReader<WaylandDisplayCreated>,
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
        .run_if(on_event::<WaylandDisplayCreated>())
}
pub fn client_name(id: &ClientId) -> String {
    let name = format!("{:?}", id)[21..35].to_string();
    format!("client@{}", name)
}

pub fn set_signal_handler() {
    use nix::sys::signal;
    extern "C" fn handle_sigsegv(_: i32) {
        std::env::set_var("RUST_BACKTRACE", "1");
        panic!("signal::SIGSEGV {}", anyhow!("").backtrace());
    }
    extern "C" fn handle_sig(s: i32) {
        std::env::set_var("RUST_BACKTRACE", "1");
        panic!("signal {} {}", s, anyhow!("").backtrace());
    }
    unsafe {
        signal::sigaction(
            signal::SIGILL,
            &signal::SigAction::new(
                signal::SigHandler::Handler(handle_sig),
                signal::SaFlags::SA_NODEFER,
                signal::SigSet::all(),
            ),
        )
        .unwrap();
        signal::sigaction(
            signal::SIGSEGV,
            &signal::SigAction::new(
                signal::SigHandler::Handler(handle_sigsegv),
                signal::SaFlags::SA_NODEFER,
                signal::SigSet::empty(),
            ),
        )
        .unwrap();
    }
}

impl DWay {
    pub fn insert<T>(&mut self, entity: Entity, f: impl EntityFactory<T>) -> EntityMut {
        let world = self.world_mut();
        f.insert(world, entity)
    }
    pub fn spawn<T>(&mut self, f: impl EntityFactory<T>) -> EntityMut {
        f.spawn(self.world_mut())
    }
}

pub trait EntityFactory<T> {
    fn spawn(self, world: &mut World) -> EntityMut<'_>
    where
        Self: Sized;
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_>;

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
    fn spawn(self, world: &mut World) -> EntityMut {
        world.spawn(self)
    }

    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert(self);
        entity_mut
    }
}
impl<T: FnOnce() -> B, B: Bundle> EntityFactory<()> for T {
    fn spawn(self, world: &mut World) -> EntityMut {
        world.spawn(self())
    }

    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert(self());
        entity_mut
    }
}
impl<T: FnOnce(&mut World) -> B, B: Bundle> EntityFactory<(&mut World,)> for T {
    fn spawn(self, world: &mut World) -> EntityMut {
        let bundle = self(world);
        world.spawn(bundle)
    }

    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let bundle = self(world);
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert(bundle);
        entity_mut
    }
}
impl<'l, 'd, T, F, B> EntityFactory<(T,)> for (New<T>, &'l mut DataInit<'d, DWay>, F)
where
    F: FnOnce(T) -> B,
    B: Bundle,
    T: WlResource + 'static,
    DWay: wayland_server::Dispatch<T, Entity>,
{
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let (resource, data_init, f) = self;
        let mut entity_mut = world.entity_mut(entity);
        let object = data_init.init(resource, entity_mut.id());
        debug!(entity=?entity_mut.id(),object=%wayland_server::Resource::id(&object),"new wayland object");
        entity_mut.insert(f(object));
        entity_mut
    }

    fn spawn(self, world: &mut World) -> EntityMut<'_>
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
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let (resource, data_init, f) = self;
        let object = data_init.init(resource, entity);
        debug!(entity=?entity,object=%wayland_server::Resource::id(&object),"new wayland object");
        let bundle: B = f(object, world);
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert(bundle);
        entity_mut
    }

    fn spawn(self, world: &mut World) -> EntityMut<'_>
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
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let mut entity = self.inner.insert(world, entity);
        entity.set_parent(self.parent);
        entity
    }

    fn spawn(self, world: &mut World) -> EntityMut
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
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        assert!(
            !world.entity_mut(entity).contains::<C>(),
            "component {} already exist in entity {entity:?}",
            type_name::<C>()
        );
        self.inner.insert(world, entity)
    }

    fn spawn(self, world: &mut World) -> EntityMut
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
    fn insert(self, world: &mut World, entity: Entity) -> EntityMut<'_> {
        let target = self.target;
        let entity_mut = self.inner.insert(world, entity);
        let entity = entity_mut.id();
        let command = ConnectCommand::<R>::new(entity, target);
        trace!(
            "connect ({:?})-[{}]->({:?})",
            entity,
            type_name::<R>(),
            target
        );
        command.write(world);
        world.entity_mut(entity)
    }

    fn spawn(self, world: &mut World) -> EntityMut
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
        command.write(world);
        world.entity_mut(entity)
    }
}
