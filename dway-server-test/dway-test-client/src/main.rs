mod bevy_app;
mod gtk4_app;
mod qt5_app;
mod winit_app;

mod cxxqt_object;

use clap::{Parser, ValueEnum};
use gtk4_app::gtk4_app;
use qt5_app::qt5_app;
use tokio::runtime::Runtime;
use winit_app::winit_app;

pub trait Client {
    fn operate(&mut self, op: ClientOperate);
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum ClientOperate {
    CreateWindow,
    CloseWindow,
    CreatePopup,
    ClosePopup,
    Snapshot,
    Quit,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default, Debug)]
pub enum ClientFramework {
    Gtk4,
    Qt5,
    #[default]
    Winit,
}

#[derive(Parser, Clone, Debug)]
#[command(version, about, long_about = None)]
pub struct ClientOption {
    #[arg(short, long)]
    pub framework: Option<ClientFramework>,
    #[arg(short, long)]
    pub operates: Vec<ClientOperate>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info,dway_test_client=debug")
        .init();

    let rt = Runtime::new().unwrap();
    let _guard = rt.enter();

    let opts = ClientOption::parse();

    match opts.framework.unwrap_or(ClientFramework::Winit) {
        ClientFramework::Gtk4 => gtk4_app(opts),
        ClientFramework::Qt5 => qt5_app(),
        ClientFramework::Winit => winit_app(),
    };
}
