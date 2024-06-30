use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

pub fn qt5_app() {
    // Create the application and engine
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/com/dway_srver_test/qt5_client/qml/main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}

