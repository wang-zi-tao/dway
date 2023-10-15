use bevy::{
    log::Level,
    prelude::{Plugin, Resource},
    reflect::Reflect,
};
use std::{io::stdout, num::NonZeroUsize};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

#[derive(Resource, Reflect)]
pub struct LoggerCache {
    pub limit: Option<NonZeroUsize>,
}
impl Default for LoggerCache {
    fn default() -> Self {
        todo!()
    }
}

pub struct DWayLogPlugin {
    pub filter: String,
    pub level: Level,
}

impl Default for DWayLogPlugin {
    fn default() -> Self {
        Self {
            filter: Default::default(),
            level: Level::INFO,
        }
    }
}

impl Plugin for DWayLogPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let _ = std::fs::create_dir(".output");
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(".output/dway_log.log")
            .unwrap();
        // let (log_file, log_file_guard) = tracing_appender::non_blocking(file);
        // let (log_stdout, log_stdout_guard) = tracing_appender::non_blocking(std::io::stderr());
        let default_filter = { format!("{},{}", self.level, self.filter) };

        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::new(&default_filter))
            .with(tracing_subscriber::fmt::Layer::new().with_writer(file))
            .with(tracing_subscriber::fmt::Layer::new().with_writer(std::io::stderr));

        let _ = bevy::utils::tracing::subscriber::set_global_default(subscriber);
    }
}
