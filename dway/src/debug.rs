use anyhow::Result;
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
#[cfg(feature = "dump_system_graph")]
use bevy_mod_debugdump::schedule_graph;
use std::path::{Path, PathBuf};

#[cfg(feature = "dump_system_graph")]
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

#[cfg(feature = "dump_system_graph")]
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
    dump_schedule(app, "Update", Update)?;
    dump_schedule(app, "PostUpdate", PostUpdate)?;
    dump_schedule(app, "Last", Last)?;
    Ok(())
}

pub fn print_resources(world: &mut World) {
    let components = world.components();
    let mut r: Vec<_> = world
        .storages()
        .resources
        .iter()
        .map(|(id, _)| id)
        .chain(world.storages().non_send_resources.iter().map(|(id, _)| id))
        .map(|id| components.get_info(id).unwrap())
        .collect();
    r.sort_by_key(|info| info.name());
    r.iter().for_each(|info| {
        debug!(
            "resource: [{:X?}] name: {} is_sync:{}",
            info.type_id(),
            info.name(),
            info.is_send_and_sync(),
        );
    });
}

pub fn print_debug_info(query: Query<(Entity, &Node, &Interaction)>, mut commands: Commands) {
    // for (entity, node, interaction) in &query {
    //     if *interaction == Interaction::Pressed {
    //         debug!(?node,?interaction,"mouse press on {entity:?}");
    //         commands.entity(entity).log_components();
    //     }
    // }
}
