use std::sync::atomic::{AtomicU32, Ordering};

pub static SERIAL: AtomicU32 = AtomicU32::new(0);
pub fn next_serial() -> u32 {
    SERIAL.fetch_add(1, Ordering::SeqCst)
}
