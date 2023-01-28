use slog::{o, Drain, Level, Logger};

pub fn logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator)
        .use_local_timestamp()
        .build()
        .fuse();
    let drain = slog::Filter::new(drain, |l| {
        l.module().starts_with("dway_server")  || l.level() > Level::Trace
    }).filter_level(Level::Debug)
    .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    Logger::root(drain, o!())
}
