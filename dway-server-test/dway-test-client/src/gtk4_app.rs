use std::time::Duration;

use bevy::time::Timer;
use gtk4::{gdk::ffi::{gdk_drag_begin, gdk_paintable_snapshot}, gio::Notification, glib::{self, ffi::{GTimer, _GTimer}, timeout_add, timeout_add_local}, prelude::*, Align, Application, ApplicationWindow, Button};

use crate::ClientOption;

pub fn gtk4_app(opts: ClientOption) {
    let application = Application::builder().build();

    application.connect_activate(build_ui);

    let mut operate_rev = opts.operates.clone();
    operate_rev.reverse();

    let app_clone = application.clone();
    timeout_add_local(Duration::from_secs(1), move ||{
        dbg!("timer");
        if let Some(ops) = operate_rev.pop(){
            match ops{
                crate::ClientOperate::CreateWindow => todo!(),
                crate::ClientOperate::CloseWindow => todo!(),
                crate::ClientOperate::CreatePopup => todo!(),
                crate::ClientOperate::ClosePopup => todo!(),
                crate::ClientOperate::Snapshot => {
                },
                crate::ClientOperate::Quit => app_clone.quit(),
            };
        }
        glib::ControlFlow::Continue
    });

    let exit_code= application.run();
    if exit_code.value()!=0 {
        panic!("gtk4 exit with code: {}", exit_code.value());
    }
}

fn build_ui(application: &Application) {
    let button = Button::builder()
        .label("Open Dialog")
        .halign(Align::Center)
        .valign(Align::Center)
        .build();

    let window = ApplicationWindow::builder()
        .application(application)
        .title("Dialog Example")
        .default_width(350)
        .default_height(70)
        .child(&button)
        .visible(true)
        .build();
}

