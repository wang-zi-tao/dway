use std::time::SystemTime;

pub mod grab;
pub mod keyboard;
pub mod pointer;
pub mod seat;
pub mod touch;

pub(crate) fn time() -> u32 {
    SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() as u32
}
