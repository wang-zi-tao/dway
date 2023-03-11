use bevy::{ecs::query::QueryEntityError, prelude::*};

pub trait QueryResultExt<T> {
    fn ok_or_log_error(self) -> Option<T>;
}
impl<T> QueryResultExt<T> for Result<T, QueryEntityError> {
    fn ok_or_log_error(self) -> Option<T> {
        match self {
            Ok(o) => Some(o),
            Err(e) => {
                error!("{e}");
                None
            }
        }
    }
}
