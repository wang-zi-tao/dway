use smithay::backend::session::Session;

// use crate::wayland::backend::udev::UDevBackend;

use self::winit::WinitBackend;

pub mod winit;
// pub mod udev;

#[derive(Debug)]
pub enum Backend{
    // UDev(UDevBackend),
    Winit(WinitBackend),
    Headless,
}
impl Backend {
    pub fn new() -> Backend {
        todo!()
    }
    pub fn seat_name(&self) -> String {
        match self{
            // Backend::UDev(u) => u.session.seat(),
            Backend::Winit(_) => "winit".into(),
            _ => "".into(),
        }
    }
}
