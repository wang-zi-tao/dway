use bevy::prelude::*;

#[cfg(feature = "dump_system_graph")]
mod dump_system_graph {
    use bevy::prelude::*;

    pub fn save_graph(
        dir: impl AsRef<std::path::Path>,
        name: &str,
        dot: &str,
    ) -> anyhow::Result<()> {
        use std::{path::PathBuf, process::Command};

        if !dir.as_ref().exists() {
            std::fs::create_dir_all(&dir)?;
        }

        let mut path = PathBuf::from(dir.as_ref());
        path.push(&format!("{name:?}"));
        path.set_extension("dot");

        let dot_path = path.clone();
        std::fs::write(&dot_path, dot)?;

        path.set_extension("svg");
        let svg_path = path;

        let _ = Command::new("dot")
            .args([
                "-Tsvg",
                &dot_path.to_string_lossy(),
                "-o",
                &svg_path.to_string_lossy(),
            ])
            .spawn();
        info!("create graph at {:?}", &svg_path);

        Ok(())
    }

    pub fn dump_schedule(
        app: &mut App,
        schedule_label: impl bevy::ecs::schedule::ScheduleLabel,
    ) -> anyhow::Result<()> {
        let schedule_name = format!("{schedule_label:?}");

        let dot = bevy_mod_debugdump::schedule_graph_dot(
            app,
            schedule_label,
            &bevy_mod_debugdump::schedule_graph::Settings::default(),
        );
        save_graph(".output/schedule", &schedule_name, &dot)?;

        Ok(())
    }

    pub fn dump_render_graph(app: &mut App) -> anyhow::Result<()> {
        use std::path::Path;

        info!("dumping render graph at .output/render_graph");
        if !Path::new(".output/render_graph").exists() {
            std::fs::create_dir(".output/render_graph")?;
        }

        let dot = bevy_mod_debugdump::render_graph_dot(
            app,
            &bevy_mod_debugdump::render_graph::Settings::default(),
        );

        save_graph(".output/render_graph", "RenderGraph", &dot)?;

        Ok(())
    }

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
        dump_render_graph(app)?;
        Ok(())
    }
}

#[cfg(feature = "dump_system_graph")]
pub use dump_system_graph::dump_schedules_system_graph;

pub struct WrapDebugName(DebugName);

impl PartialEq for WrapDebugName {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for WrapDebugName {}

impl PartialOrd for WrapDebugName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl Ord for WrapDebugName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
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
    r.sort_by_key(|info| WrapDebugName(info.name()));
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

pub struct DebugRenderPlugin;

#[cfg(feature = "debug_render")]
pub mod render_doc {
    use renderdoc::*;
    pub type Version = V110;
    pub struct RenderDocContext {
        pub rd: RenderDoc<Version>,
    }
    pub fn start_render_doc() -> RenderDocContext {
        let mut rd: RenderDoc<V110> = RenderDoc::new().expect("Unable to connect");
        rd.trigger_multi_frame_capture(16);
        rd.set_log_file_path_template(".output/renderdoc_capture.rdc");

        RenderDocContext { rd }
    }
}

#[cfg(feature = "debug_render")]
pub use render_doc::start_render_doc;
