use anyhow::Result;
use bevy::{
    app::RunFixedUpdateLoop,
    ecs::schedule::ScheduleLabel,
    prelude::*,
    render::{Extract, Render, RenderApp},
};
use bevy_mod_debugdump::schedule_graph;
use std::path::{Path, PathBuf};

pub fn dump_schedule(app: &mut App, name: &str, schedule_label: impl ScheduleLabel) -> Result<()> {
    let dot = bevy_mod_debugdump::schedule_graph_dot(
        app,
        schedule_label,
        &schedule_graph::Settings::default(),
    );
    let mut path = PathBuf::from(".output/schedule");
    path.push(name);
    path.set_extension("dot");
    std::fs::write(&path, dot)?;
    Ok(())
}

pub fn dump_schedules_system_graph(app: &mut App) -> Result<()> {
    info!("dumping system graph at .output/schedule");
    if !Path::new(".output/schedule").exists() {
        std::fs::create_dir(".output/schedule")?;
    }

    dump_schedule(app, "Main", Main)?;
    dump_schedule(app, "PreStartUp", PreStartup)?;
    dump_schedule(app, "Startup", Startup)?;
    dump_schedule(app, "PostStartup", PostStartup)?;
    dump_schedule(app, "First", First)?;
    dump_schedule(app, "PreUpdate", PreUpdate)?;
    dump_schedule(app, "StateTransition", StateTransition)?;
    dump_schedule(app, "RunFixedUpdateLoop", RunFixedUpdateLoop)?;
    dump_schedule(app, "Update", Update)?;
    dump_schedule(app, "PostUpdate", PostUpdate)?;
    dump_schedule(app, "Last", Last)?;
    Ok(())
}
