use std::any::type_name;

use bevy::prelude::error;
use wayland_server::WEnum;

pub mod fail;
pub mod file;
pub mod rect;
pub mod serial;

pub fn unimplemented<T>() -> T {
    unimplemented!()
}

pub fn unwrap_wl_enum<T>(e: WEnum<T>) -> Option<T> {
    match e {
        WEnum::Value(v) => Some(v),
        WEnum::Unknown(v) => {
            error!("unknown wayland {}({})", type_name::<T>(), v);
            None
        }
    }
}
