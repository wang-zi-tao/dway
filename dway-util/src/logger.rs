use bevy::{app::Update, ecs::system::NonSendMut, log::Level, prelude::Plugin};
use smallvec::SmallVec;
use std::{collections::VecDeque, sync::mpsc};
use tracing_subscriber::{fmt::MakeWriter, prelude::__tracing_subscriber_SubscriberExt, EnvFilter};

#[derive(Debug)]
pub struct LogLine {
    pub data: Vec<u8>,
}

pub struct LoggerCache {
    pub limit: usize,
    pub rx: Option<mpsc::Receiver<LogLine>>,
    pub lines: VecDeque<LogLine>,
}
impl Default for LoggerCache {
    fn default() -> Self {
        Self {
            limit: 1024,
            rx: None,
            lines: Default::default(),
        }
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

pub fn revceive_log_system(mut cache: NonSendMut<LoggerCache>) {
    if let Some(rx) = cache.rx.as_ref() {
        let log_list = rx.try_iter().collect::<SmallVec<[ _;16 ]>>();
        if !log_list.is_empty() {
            let LoggerCache { limit, lines, .. } = &mut *cache;
            for log in log_list {
                if *limit <= lines.len() {
                    lines.pop_front();
                }
                lines.push_back(log);
            }
        }
    }
}

#[derive(Clone)]
struct LoggerWritter {
    pub tx: mpsc::Sender<LogLine>,
}

impl std::io::Write for LoggerWritter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.tx.send(LogLine {
            data: buf.to_vec(),
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for LoggerWritter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Plugin for DWayLogPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_non_send_resource::<LoggerCache>();
        app.add_systems(Update, revceive_log_system);

        let _ = std::fs::create_dir(".output");
        let file_appender = tracing_appender::rolling::hourly(".output", "dway_tty.log");
        let (log_file, log_file_guard) = tracing_appender::non_blocking(file_appender);
        app.insert_non_send_resource(log_file_guard);
        let default_filter = format!("{},{}", self.level, self.filter);

        let (tx, rx) = mpsc::channel();
        let mut cache = app.world.non_send_resource_mut::<LoggerCache>();
        cache.rx = Some(rx);

        let subscriber = tracing_subscriber::registry()
            .with(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| EnvFilter::new(&default_filter)),
            )
            .with(tracing_subscriber::fmt::Layer::new().with_writer(std::io::stderr))
            .with(tracing_subscriber::fmt::Layer::new().with_writer(log_file))
            .with(tracing_subscriber::fmt::Layer::new().with_writer(LoggerWritter { tx }));

        let _ = bevy::utils::tracing::subscriber::set_global_default(subscriber);
        // log_panics::init()
    }
}
