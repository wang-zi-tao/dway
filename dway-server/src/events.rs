use crate::{input::grab::ResizeEdges, prelude::*, util::rect::IRect};

pub struct Bind<T: WlResource> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
pub struct Insert<T> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
impl<T: Component> Event for Insert<T> {}
impl<T> Insert<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

pub struct Destroy<T> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
impl<T: Component> Event for Destroy<T> {}
impl<T> Destroy<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

#[derive(Event)]
pub struct ResizeWindow {
    pub entity: Entity,
    pub delta: IVec2,
    pub size: IVec2,
}

#[derive(Event)]
pub struct MoveWindow {
    pub entity: Entity,
    pub delta: IVec2,
}

#[derive(Event)]
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

#[derive(Event)]
pub struct MoveRequest(pub Entity);

#[derive(Event)]
pub struct ResizeRequest(pub Entity);

pub struct EventPlugin;
impl Plugin for EventPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ResizeWindow>();
        app.add_event::<MoveWindow>();
        app.add_event::<MoveRequest>();
        app.add_event::<ResizeRequest>();
        app.add_event::<WindowAction>();
    }
}
