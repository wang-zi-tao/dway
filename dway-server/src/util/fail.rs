use crate::{prelude::*, wl::surface::WlSurface, xdg::DWayWindow};
use bevy::ecs::query::QueryEntityError;

pub fn get_type_name_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

macro_rules! try_get {
    ($query:expr,$entity:expr) => {
        {
            let entity = $entity;
            match $query.get(entity) {
                Ok(r)=>Some(r),
                Err(e)=>{
                    error!(query=%get_type_name_of(&$query),entity=?entity,"{e}");
                    None
                }
            }
        }
    };
    ($query:expr,mut $entity:expr) => {
        {
            let entity = $entity;
            match $query.get_mut(entity) {
                Ok(r)=>Some(r),
                Err(e)=>{
                    error!(query=%get_type_name_of(&$query),entity=?entity,"{e}");
                    None
                }
            }
        }
    };
}
pub fn system(e: &Entity, mut q: Query<&WlSurface>) {
    let r = try_get!(q, *e);
    let r = try_get!(q,mut*e);
}
