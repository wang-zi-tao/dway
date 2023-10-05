use std::num::NonZeroUsize;

use bevy::{prelude::Resource, reflect::Reflect, utils::tracing::instrument::WithSubscriber};
use tracing_subscriber::Registry;

#[derive(Resource, Reflect)]
pub struct LoggerCache {
    pub limit: Option<NonZeroUsize>,
}
impl Default for LoggerCache {
    fn default() -> Self {
        todo!()
    }
}
