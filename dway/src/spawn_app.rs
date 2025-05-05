use bevy::prelude::*;
use dway_server::apps::launchapp::RunCommandRequest;

use crate::opttions::DWayOption;

pub fn spawn_apps_on_launch(opts: &DWayOption, event_sender: &mut EventWriter<RunCommandRequest>) {
    if opts.exec.len() > 0 {
        let command = opts.exec[0].clone();
        let args = opts.exec.iter().skip(1).cloned().collect();
        event_sender.send(RunCommandRequest {
            command,
            args,
            ..Default::default()
        });
    }
}
