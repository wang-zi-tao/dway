use crate::{prelude::*, util::rect::IRect};

pub struct Bind<T: WlResource> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
pub struct Insert<T> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}

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

impl<T> Destroy<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

pub struct ResizeWindow {
    pub entity: Entity,
    pub delta: IVec2,
    pub size: IVec2,
}
pub struct MoveWindow {
    pub entity: Entity,
    pub delta: IVec2,
}

pub enum WindowAction{
    Close(Entity),
    Maximize(Entity),
    UnMaximize(Entity),
    Fullscreen(Entity),
    UnFullscreen(Entity),
    Minimize(Entity),
    UnMinimize(Entity),
    SetRect(Entity,IRect),
}

pub struct MoveRequest(pub Entity);
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
