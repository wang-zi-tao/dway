use super::DesktopEntry;
use crate::prelude::*;
use bevy::utils::HashMap;
use derive_builder::Builder;
use std::process::{self, Command};

#[derive(Debug, Event, Builder)]
pub struct LaunchAppRequest {
    pub app_entity: Entity,
    #[builder(setter(into), default)]
    pub file_name: Option<String>,
    #[builder(setter(into), default)]
    pub url: Option<String>,
}

#[derive(Debug, Event, Builder, Default)]
pub struct RunCommandRequest {
    #[builder(setter(into))]
    pub command: String,
    #[builder(setter(into), default)]
    pub args: Vec<String>,
    #[builder(setter(into), default)]
    pub envs: HashMap<String, String>,
}

pub fn run_command_system(
    mut event: EventReader<RunCommandRequest>,
    server_query: Query<&DWayServer>,
) {
    for RunCommandRequest {
        command,
        args,
        envs,
    } in event.read()
    {
        let Some(compositor) = server_query.iter().next() else {
            continue;
        };
        let mut command = Command::new(command);
        command.args(args).envs(envs);
        compositor.spawn_process(command);
    }
}

impl LaunchAppRequest {
    pub fn new(app_entity: Entity) -> Self {
        Self {
            app_entity,
            file_name: None,
            url: None,
        }
    }
}

pub fn launch_app_system(
    mut event: EventReader<LaunchAppRequest>,
    app_query: Query<&DesktopEntry>,
    server_query: Query<&DWayServer>,
) {
    for LaunchAppRequest {
        app_entity,
        file_name,
        url,
    } in event.read()
    {
        let Ok(desktop_entry) = app_query.get(*app_entity) else {
            continue;
        };

        let Some(exec) = desktop_entry.exec() else {
            continue;
        };

        let file_value = file_name.as_ref().map(|s| s.as_str()).unwrap_or("");
        let url_value = url.as_ref().map(|s| s.as_str()).unwrap_or("");
        let entry_value = &desktop_entry.path.to_string_lossy();
        let mut exec = exec.to_owned();
        exec = exec.replace("%f", file_value);
        exec = exec.replace("%F", file_value);
        exec = exec.replace("%u", url_value);
        exec = exec.replace("%U", url_value);
        exec = exec.replace("%k", entry_value);
        exec = exec.replace("%K", entry_value);

        let Some(compositor) = server_query.iter().next() else {
            continue;
        };
        let mut command = process::Command::new("sh");
        command.arg("-c").arg(exec);
        if let Some(cwd) = desktop_entry.current_dir() {
            command.current_dir(cwd);
        }
        compositor.spawn_process(command);
    }
}
