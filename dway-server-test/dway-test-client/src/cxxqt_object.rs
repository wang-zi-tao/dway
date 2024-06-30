use core::pin::Pin;
use std::process::exit;
use cxx_qt_lib::{QCoreApplication, QGuiApplication, QQmlApplicationEngine, QString};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(i32, number)]
        type UiObject = super::UiObjectRust;
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        fn button_exit(self: &UiObject);
    }
}

#[derive(Default)]
pub struct UiObjectRust {
    number: i32,
}

impl qobject::UiObject {
    pub fn button_exit(self: &Self) {
        exit(0);
    }
}
