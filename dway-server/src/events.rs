use crate::{prelude::*, resource::ResourceWrapper};

pub struct Bind<T: WlResource> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}
pub struct Insert<T: ResourceWrapper> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}

impl<T: ResourceWrapper> Insert<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            phase: std::marker::PhantomData,
        }
    }
}

pub struct Destroy<T: ResourceWrapper> {
    pub entity: Entity,
    pub phase: std::marker::PhantomData<T>,
}

impl<T: ResourceWrapper> Destroy<T> {
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
pub struct MoveWindow{
    pub entity: Entity,
    pub delta:IVec2,
}
pub struct MoveRequest( pub Entity );
pub struct ResizeRequest( pub Entity );

pub struct EventPlugin;
impl Plugin for EventPlugin{
    fn build(&self, app: &mut App) {
        app.add_event::<ResizeWindow>();
        app.add_event::<MoveWindow>();
        app.add_event::<MoveRequest>();
        app.add_event::<ResizeRequest>();
    }
}
