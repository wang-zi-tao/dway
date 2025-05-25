pub mod grab;
pub mod keyboard;
pub mod pointer;
pub mod seat;
pub mod touch;
pub mod textinput;

use std::time::SystemTime;

pub(crate) fn time() -> u32 {
    SystemTime::UNIX_EPOCH.elapsed().unwrap().as_millis() as u32
}
