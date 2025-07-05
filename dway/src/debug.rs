use bevy::prelude::*;

#[cfg(feature = "dump_system_graph")]
pub fn dump_schedule(
    app: &mut App,
    schedule_label: impl bevy::ecs::schedule::ScheduleLabel,
) -> anyhow::Result<()> {
    use std::path::PathBuf;

    let mut path = PathBuf::from(".output/schedule");
    path.push(&format!("{schedule_label:?}"));
    path.set_extension("dot");

    let dot = bevy_mod_debugdump::schedule_graph_dot(
        app,
        schedule_label,
        &bevy_mod_debugdump::schedule_graph::Settings::default(),
    );
    std::fs::write(&path, dot)?;

    let mut svg_path = path.clone();
    svg_path.set_extension("svg");
    let _ = std::process::Command::new("dot")
        .args([
            "-Tsvg",
            &path.to_string_lossy(),
            "-o",
            &svg_path.to_string_lossy(),
        ])
        .spawn();
    info!("create system graph at {:?}", &svg_path);

    Ok(())
}

#[cfg(feature = "dump_system_graph")]
pub fn dump_schedules_system_graph(app: &mut App) -> anyhow::Result<()> {
    use std::path::Path;

    info!("dumping system graph at .output/schedule");
    if !Path::new(".output/schedule").exists() {
        std::fs::create_dir(".output/schedule")?;
    }

    dump_schedule(app, Main)?;
    dump_schedule(app, Startup)?;
    dump_schedule(app, PostStartup)?;
    dump_schedule(app, First)?;
    dump_schedule(app, PreUpdate)?;
    dump_schedule(app, StateTransition)?;
    dump_schedule(app, Update)?;
    dump_schedule(app, PostUpdate)?;
    dump_schedule(app, Last)?;
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

#[cfg(feature = "dhat-heap")]
pub fn memory_profiler() -> dhat::Profiler {
    dhat::Profiler::new_heap()
}

#[cfg(feature = "pprof")]
mod pprof {
    use std::{fs::File, process::Command};

    use pprof::ProfilerGuard;

    use crate::info;

    pub struct PprofGuard(ProfilerGuard<'static>);
    impl Drop for PprofGuard {
        fn drop(&mut self) {
            if let Ok(report) = self.0.report().build() {
                info!("performance report: flamegraph.svg");
                {
                    let file = File::create("flamegraph.svg").unwrap();
                    report.flamegraph(file).unwrap();
                }
                let _ = Command::new("xdg-open").arg("flamegraph.svg").spawn();
            };
        }
    }

    pub fn pprof_profiler() -> PprofGuard {
        PprofGuard(
            pprof::ProfilerGuardBuilder::default()
                .frequency(1000)
                .blocklist(&["libc", "libgcc", "pthread", "vdso"])
                .build()
                .unwrap(),
        )
    }
}

#[cfg(feature = "pprof")]
pub use pprof::pprof_profiler;
