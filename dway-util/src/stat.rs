use std::time::Instant;

use log::{debug, info, warn};

pub fn memory_profile() {
    if let Some(usage) = memory_stats::memory_stats() {
        info!(
            "Current memory usage: physical: {}MiB, virtual: {}MiB",
            usage.physical_mem / 1024 / 1024,
            usage.virtual_mem / 1024 / 1024
        );
    } else {
        warn!("Couldn't get the current memory usage :(");
    }
}

pub struct PerfLog(Instant, String);
impl PerfLog {
    pub fn new(name: &str) -> Self {
        Self(Instant::now(), name.to_string())
    }
}
impl Drop for PerfLog {
    fn drop(&mut self) {
        debug!(
            "PERF: PerfLog {:?}: {}",
            self.1,
            (Instant::now() - self.0).as_millis()
        );
    }
}
