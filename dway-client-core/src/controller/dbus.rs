use anyhow::{anyhow, Result};
use dbus::{
    arg::{AppendAll, ReadAll},
    ffidisp::Connection,
};
use dway_server::macros::Resource;
use std::time::Duration;

pub struct DBusController {
    pub connection: Connection,
}

impl Default for DBusController {
    fn default() -> Self {
        Self {
            connection: Connection::new_session().unwrap(),
        }
    }
}

impl DBusController {
    pub fn method_call<A: AppendAll, R: ReadAll>(
        &self,
        bus: &str,
        path: &str,
        member: &str,
        timeout: Duration,
        args: A,
    ) -> Result<R> {
        let path = self
            .connection
            .with_path(bus, path, timeout.as_millis() as i32);
        Ok(path.method_call(bus, member, args)?)
    }
}
