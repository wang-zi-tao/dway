use std::{borrow::Cow, collections::VecDeque, sync::mpsc};

use backtrace::Backtrace;
use bevy::{
    app::{App, Update},
    ecs::{prelude::Resource, system::NonSendMut},
    log::{BoxedLayer, error, warn},
    prelude::Plugin,
};
use nix::sys::{signal, signal::Signal};
use smallvec::SmallVec;
use tracing_subscriber::{fmt::MakeWriter, Layer};

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

#[derive(Clone, Debug, Resource)]
pub struct DWayLogSetting {
    pub log_dir: Cow<'static, str>,
    pub log_file: Cow<'static, str>,
}

impl Default for DWayLogSetting {
    fn default() -> Self {
        Self {
            log_dir: ".output".into(),
            log_file: "dway.log".into(),
        }
    }
}

pub fn revceive_log_system(mut cache: NonSendMut<LoggerCache>) {
    if let Some(rx) = cache.rx.as_ref() {
        let log_list = rx.try_iter().collect::<SmallVec<[_; 16]>>();
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
        let _ = self.tx.send(LogLine { data: buf.to_vec() });
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

pub fn log_layer(app: &mut App) -> Option<BoxedLayer> {
    let settings = app
        .world_mut()
        .get_resource_or_insert_with::<DWayLogSetting>(Default::default)
        .clone();
    let _ = std::fs::create_dir(&*settings.log_dir);
    let file_appender = tracing_appender::rolling::hourly(&*settings.log_dir, &*settings.log_file);
    let (log_file, log_file_guard) = tracing_appender::non_blocking(file_appender);
    app.insert_non_send_resource(log_file_guard);

    let layers = (tracing_subscriber::fmt::Layer::new().with_writer(log_file))
        .and_then(tracing_journald::Layer::new().unwrap());

    if let Some(mut cache) = app.world_mut().get_non_send_resource_mut::<LoggerCache>() {
        let (tx, rx) = mpsc::channel();
        cache.rx = Some(rx);
        Some(Box::new(layers.and_then(
            tracing_subscriber::fmt::Layer::new().with_writer(LoggerWritter { tx }),
        )))
    } else {
        Some(Box::new(layers))
    }
}

pub struct DWayLogPlugin;
impl Plugin for DWayLogPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<DWayLogSetting>();
        app.init_non_send_resource::<LoggerCache>();
        app.add_systems(Update, revceive_log_system);
        install_panic_hook();
        install_signal_handler();
    }
}

pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("<unnamed>");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &**s,
                None => "Box<Any>",
            },
        };

        let backtrace = Backtrace::default();
        let localtion_string = info
            .location()
            .map(|l| format!(": {}:{}", l.file(), l.line()));
        error!(
            target: "panic", "thread '{}' panicked at '{}'{}{:?}",
            thread,
            msg,
            localtion_string.as_deref().unwrap_or(""),
            backtrace
        );
    }));
}

extern "C" fn handle_sig(s: i32) {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    error!(
        "signal {} {:?}",
        Signal::try_from(s)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| s.to_string()),
        backtrace::Backtrace::new()
    );
}

fn register_signal(signal: Signal) {
    unsafe {
        if let Err(err) = signal::sigaction(
            signal,
            &signal::SigAction::new(
                signal::SigHandler::Handler(handle_sig),
                signal::SaFlags::empty(),
                signal::SigSet::empty(),
            ),
        ) {
            warn!(
                "failed to registry signal handle for signal {:?}: {err}",
                signal
            );
        };
    };
}

pub fn install_signal_handler() {
    register_signal(Signal::SIGKILL);
    register_signal(Signal::SIGSEGV);
}
