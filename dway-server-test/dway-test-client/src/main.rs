mod bevy_app;
mod gtk4_app;
mod qt5_app;
mod winit_app;

mod cxxqt_object;

use clap::{Parser, ValueEnum};
use qt5_app::qt5_app;
use tokio::runtime::Runtime;
use winit_app::winit_app;
use gtk4_app::gtk4_app;

pub trait Client{
    fn operate(&mut self, op: ClientOperate);
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ClientOperate{
    CreateWindow,
    CloseWindow,
    CreatePopup,
    ClosePopup,
    Snapshot,
    Quit
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Debug)]
pub enum ClientFramework {
    Gtk4,
    #[default]
    Qt5,
    Winit,
}

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct ClientOption {
    #[arg(value_enum, default_value_t=ClientFramework::Gtk4)]
    pub framework: ClientFramework,
    #[arg(value_enum)]
    pub operates: Vec<ClientOperate>,
}

fn main() {
    let rt = Runtime::new().unwrap();
    let _guard = rt.enter();

    let opts = ClientOption::parse();
    dbg!(&opts);

    match opts.framework{
        ClientFramework::Gtk4 => gtk4_app(opts),
        ClientFramework::Qt5 => qt5_app(),
        ClientFramework::Winit => winit_app(),
    };
}
