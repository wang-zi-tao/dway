#![feature(ptr_metadata)]
use std::{
    any::type_name,
    io,
    os::{fd::AsRawFd, unix::net::UnixStream},
    process::{self, Stdio},
    ptr::null_mut,
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy::{
    ecs::{query::WorldQuery, world::EntityMut},
    prelude::*,
    utils::Instant,
};
use calloop::{
    generic::Generic, EventLoop, EventSource, Interest, Mode, Poll, PostAction, Readiness, Token,
    TokenFactory,
};
use eventloop::ListeningSocketEvent;
use failure::Error;
use inlinable_string::InlinableString;
use wayland_server::{
    backend::{ClientData, ClientId, DisconnectReason},
    protocol::wl_output,
    DataInit, ListeningSocket, New,
};

use crate::{geometry::GeometryPlugin, schedule::DWayServerSet};
mod prelude;
pub mod client;
pub mod dispatch;
pub mod display;
pub mod eventloop;
pub mod events;
pub mod geometry;
pub mod input;
pub mod render;
pub mod resource;
pub mod schedule;
pub mod util;
pub mod wl;
pub mod x11;
pub mod xdg;
pub mod zxdg;

#[derive(Debug, Default)]
pub struct ClientState;
impl ClientData for ClientState {
    /// Notification that a client was initialized
    fn initialized(&self, _client_id: ClientId) {}
    /// Notification that a client is disconnected
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

#[derive(Resource)]
pub struct DWayCalloopData {
    pub state: Box<DWay>,
    pub display: Box<wayland_server::Display<DWay>>,
}

pub struct DWay {
    pub world: *mut World,
    pub socket_name: InlinableString,
    pub display_number: Option<usize>,
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
        trace!(?entity,resource=?wayland_server::Resource::id(object),"destroy wayland object");
        world.entity_mut(entity).despawn_recursive();
    }
    pub fn init_object<T, C, F>(
        &mut self,
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
        trace!(?entity,object=?wayland_server::Resource::id(&object),"spawn object");
        entity_command.insert(f(object));
        entity
    }
    pub fn insert_object_bundle<T, C, B, F>(
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
        F: FnOnce(T) -> (C, B),
    {
        let world = self.world_mut();
        assert!(
            !world.entity_mut(entity).contains::<C>(),
            "component {} already exist in entity {entity:?}",
            type_name::<C>()
        );
        let object = data_init.init(resource, entity);
        trace!(?entity,object=?wayland_server::Resource::id(&object),"insert object");
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
        trace!(?entity,object=?wayland_server::Resource::id(&object),"insert object");
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
        trace!(parent=?parent,?entity,object=?wayland_server::Resource::id(&object),"spawn object");
        entity_command.insert(f(object));
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
        trace!(parent=?parent,?entity,object=?wayland_server::Resource::id(&object),"spawn object");
        entity_command.insert(f(object));
        world.entity_mut(parent).add_child(entity);
        entity
    }
    pub fn get_entity(object: &impl wayland_server::Resource) -> Entity {
        *object.data::<Entity>().unwrap()
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
        self.world_mut().get_entity_mut(entity).map(EntityMut::despawn);
    }
    pub fn despawn_object(&mut self, entity: Entity, id:wayland_backend::server::ObjectId) {
        trace!(entity=?entity,resource=%id,"despawn object");
        self.world_mut().get_entity_mut(entity).map(EntityMut::despawn);
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
    pub fn query<B, F, R>(&mut self, entity:Entity, f: F) -> R
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
pub struct DWayEventLoop(pub EventLoop<'static, DWayCalloopData>);

#[derive(Default)]
pub struct DWayServerPlugin;
impl Plugin for DWayServerPlugin {
    fn build(&self, app: &mut App) {
        let mut event_loop = EventLoop::try_new().unwrap();
        let mut display = wayland_server::Display::<DWay>::new().unwrap();

        let handle = Arc::new(display.handle());
        handle.create_global::<DWay, wl_output::WlOutput, _>(4, ());
        let source = ListeningSocketEvent::new();
        let socket_name = source.filename();

        info!("listening on {}", &socket_name);
        event_loop
            .handle()
            .insert_source(
                source,
                move |client_stream, _, data: &mut DWayCalloopData| {
                    if let Err(err) = data
                        .display
                        .handle()
                        .insert_client(client_stream, Arc::new(ClientState))
                    {
                        warn!("Error adding wayland client: {}", err);
                    };
                },
            )
            .expect("Failed to init wayland socket source");
        event_loop
            .handle()
            .insert_source(
                Generic::new(
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ,
                    Mode::Level,
                ),
                |_, _, state| {
                    state.display.dispatch_clients(&mut state.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        app.insert_non_send_resource(DWayEventLoop(event_loop));
        app.insert_resource(DWayCalloopData {
            display: Box::new(display),
            state: Box::new(DWay {
                world: null_mut(),
                socket_name,
                display_number: None,
            }),
        });

        app.add_plugin(geometry::GeometryPlugin);
        app.add_plugin(schedule::DWayServerSchedulePlugin);
        app.add_plugin(events::EventPlugin);
        app.add_plugin(wl::output::WlOutputPlugin(handle.clone()));
        app.add_plugin(wl::surface::WlSurfacePlugin(handle.clone()));
        app.add_plugin(wl::buffer::WlBufferPlugin(handle.clone()));
        app.add_plugin(wl::region::WlRegionPlugin(handle.clone()));
        app.add_plugin(wl::compositor::WlCompositorPlugin(handle.clone()));
        app.add_plugin(input::seat::WlSeatPlugin(handle.clone()));
        app.add_plugin(render::DWayServerRenderPlugin);
        app.add_plugin(xdg::XdgShellPlugin(handle.clone()));
        app.add_plugin(xdg::toplevel::XdgToplevelPlugin(handle.clone()));
        app.add_plugin(xdg::popup::XdgPopupPlugin(handle.clone()));
        app.add_plugin(zxdg::outputmanager::XdgOutputManagerPlugin(handle.clone()));
        app.add_system(receive_event.in_set(DWayServerSet::Dispatch));
        app.add_startup_system(init);
    }
}
pub fn receive_event(world: &mut World) {
    let mut event_loop = world.remove_non_send_resource::<DWayEventLoop>().unwrap();
    let mut calloop = world.remove_resource::<DWayCalloopData>().unwrap();
    calloop.state.world = world as *mut World;
    let start_time = Instant::now();
    let duration = Duration::from_secs_f32(0.004);
    let end_time = start_time + duration;
    event_loop.0.dispatch(Some(duration), &mut calloop).unwrap();
    calloop
        .display
        .dispatch_clients(&mut calloop.state)
        .unwrap();
    loop {
        let now = Instant::now();
        if now > end_time {
            break;
        }
        event_loop
            .0
            .dispatch(Some(end_time - now), &mut calloop)
            .unwrap();
        calloop
            .display
            .dispatch_clients(&mut calloop.state)
            .unwrap();
    }
    calloop.display.flush_clients().unwrap();
    calloop.state.world = null_mut();
    world.insert_resource(calloop);
    world.insert_non_send_resource(event_loop);
}
pub fn init(calloop: Res<DWayCalloopData>) {
    let compositor = &calloop.state;
    let mut command = process::Command::new("gnome-calculator");
    let mut command = process::Command::new("gedit");
    let mut command = process::Command::new("gnome-system-monitor");
    let mut command = process::Command::new(
        "/home/wangzi/workspace/waylandcompositor/conrod/target/debug/examples/all_winit_glium",
    );
    let mut command= process::Command::new("alacritty");
    command.args([ "-e","htop" ]);
    // command.current_dir("/home/wangzi/workspace/waylandcompositor/conrod/");
    // let mut command = process::Command::new("/nix/store/gfn9ya0rwaffhfkpbbc3pynk247xap1h-qt5ct-1.5/bin/qt5ct");
    // let mut command = process::Command::new("/home/wangzi/workspace/waylandcompositor/wayland-rs/wayland-client/../target/debug/examples/simple_window");
    // let mut command = process::Command::new("/home/wangzi/.build/0bd4966a8a745859d01236fd5f997041598cc31-bevy/debug/examples/animated_transform");
    let mut command = process::Command::new(
        "/home/wangzi/workspace/waylandcompositor/winit_demo/target/debug/winit_demo",
    );
    command.stdout(Stdio::inherit());
    compositor.spawn(command);
}
