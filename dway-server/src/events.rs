use bevy::ecs::event::GlobalTrigger;

use crate::{input::grab::ResizeEdges, prelude::*, util::rect::IRect};

pub struct Bind<T: WlResource> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
#[derive(Component)]
pub struct Insert<T> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Message for Insert<T> {
}

impl<T: Send + Sync + 'static> Event for Insert<T> {
    type Trigger<'a> = GlobalTrigger;
}

impl<T> Insert<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

#[derive(Component)]
pub struct Destroy<T> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}

impl<T: Send + Sync + 'static> Message for Destroy<T> {
}

impl<T: Send + Sync + 'static> Event for Destroy<T> {
    type Trigger<'a> = GlobalTrigger;
}

impl<T> Destroy<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

#[derive(Message)]
pub struct ResizeWindow {
    pub entity: Entity,
    pub delta: IVec2,
    pub size: IVec2,
}

#[derive(Message, Debug)]
pub struct MoveWindow {
    pub entity: Entity,
    pub delta: IVec2,
}

#[derive(Message, Debug, Clone)]
pub enum WindowAction {
    Close(Entity),
    Maximize(Entity),
    UnMaximize(Entity),
    Fullscreen(Entity),
    UnFullscreen(Entity),
    Minimize(Entity),
    UnMinimize(Entity),
    SetRect(Entity, IRect),
    RequestMove(Entity),
    RequestResize(Entity, ResizeEdges),
}

#[derive(Message)]
pub struct MoveRequest(pub Entity);

#[derive(Message)]
pub struct ResizeRequest(pub Entity);

#[derive(Message, Deref)]
pub struct DispatchDisplay(pub Entity);

#[derive(Message, Deref)]
pub struct DispatchXWaylandDisplay(pub Entity);

#[derive(Message)]
pub struct WindowAppIdChanged {
    pub entity: Entity,
    pub app_id: String,
}

#[derive(Message)]
pub struct WindowAttachedToApp {
    pub app_entity: Entity,
    pub window_entity: Entity,
}

#[derive(Message)]
pub struct XWindowChanged {
    pub xwindow_entity: Entity,
    pub surface_entity: Option<Entity>,
}

#[derive(Message)]
pub struct XWindowAttachSurfaceRequest {
    pub xwindow_entity: Entity,
}

pub struct EventPlugin;
impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
    app.add_message::<ResizeWindow>();
    app.add_message::<MoveWindow>();
    app.add_message::<MoveRequest>();
    app.add_message::<ResizeRequest>();
    app.add_message::<WindowAction>();
    app.add_message::<DispatchDisplay>();
    app.add_message::<DispatchXWaylandDisplay>();
    app.add_message::<WindowAppIdChanged>();
    app.add_message::<WindowAttachedToApp>();
    app.add_message::<XWindowChanged>();
    app.add_message::<XWindowAttachSurfaceRequest>();
    }
}
