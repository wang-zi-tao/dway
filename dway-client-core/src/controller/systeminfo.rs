use std::time::Instant;

use dway_server::macros::{ResMut, Resource};
use sysinfo::{Disks, Networks, System};

pub struct CpuInfo {
    pub name: String,
    pub used: f32,
    pub frequency: f32,
}

#[derive(Resource)]
pub struct SystemInfo {
    system: System,
    disks: Disks,
    networks: Networks,
    last_refresh_time: Instant,
    refresh_time: Instant,
}

impl Default for SystemInfo {
    fn default() -> Self {
        let instant = Instant::now();
        Self {
            system: Default::default(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_refresh_time: instant,
            refresh_time: instant,
        }
    }
}

impl SystemInfo {
    pub fn os(&self) -> Option<String> {
        System::name()
    }

    pub fn cpu_list(&self) -> Vec<CpuInfo> {
        self.system
            .cpus()
            .iter()
            .map(|c| CpuInfo {
                name: c.name().to_string(),
                used: c.cpu_usage() / 100.0,
                frequency: c.frequency() as f32,
            })
            .collect()
    }

    pub fn cpu_usage(&self) -> f32 {
        self.system.global_cpu_info().cpu_usage() / 100.0
    }

    pub fn cpu_count(&self) -> usize {
        self.system.cpus().len()
    }

    pub fn cpu_frequency(&self) -> u64 {
        self.system.global_cpu_info().frequency()
    }

    pub fn total_memory(&self) -> u64 {
        self.system.total_memory()
    }
    pub fn used_memory(&self) -> u64 {
        self.system.used_memory()
    }
    pub fn available_memory(&self) -> u64 {
        self.system.available_memory()
    }
    pub fn total_swap(&self) -> u64 {
        self.system.total_swap()
    }
    pub fn used_swap(&self) -> u64 {
        self.system.used_swap()
    }
    pub fn free_swap(&self) -> u64 {
        self.system.free_swap()
    }

    pub fn uptime(&self) -> u64 {
        sysinfo::System::uptime()
    }

    pub fn network_upload(&self) -> u64 {
        (self
            .networks
            .iter()
            .map(|interface| interface.1.transmitted())
            .sum::<u64>() as f32
            / self.duration_second()) as u64
    }

    pub fn network_download(&self) -> u64 {
        (self
            .networks
            .iter()
            .map(|interface| interface.1.received())
            .sum::<u64>() as f32
            / self.duration_second()) as u64
    }

    pub fn duration_second(&self) -> f32 {
        (self.refresh_time - self.last_refresh_time).as_secs_f32()
    }
}

pub fn update_system_info_system(mut system_info: ResMut<SystemInfo>) {
    system_info.system.refresh_cpu();
    system_info.system.refresh_memory();
    system_info.disks.refresh_list();
    system_info.disks.refresh();
    system_info.networks.refresh_list();
    system_info.networks.refresh();
    system_info.last_refresh_time = system_info.refresh_time;
    system_info.refresh_time = Instant::now();
}

pub fn human_readable_byte(byte: u64)->String{
    if byte >> 30 !=0 {
        format!("{:.1}GiB", byte as f32 / (1<<30) as f32)
    }else if byte >> 20 != 0 {
        format!("{:.1}MiB", byte as f32 / (1<<20) as f32)
    }else if byte >> 10 != 0 {
        format!("{:.1}KiB", byte as f32 / (1<<10) as f32)
    }else{
        format!("{}B", byte)
    }
}

pub fn human_readable_fresequency(frequency: u64) -> String {
    let byte = frequency;
    if frequency > 1000_000_000 {
        format!("{:.1}GHz", byte as f32 / 1000_000_000 as f32)
    }else if frequency > 1000_000 {
        format!("{:.1}MHz", byte as f32 / 1000_000 as f32)
    }else if frequency > 1000 {
        format!("{:.1}KHz", frequency as f32 / 1000 as f32)
    }else{
        format!("{}Hz", frequency)
    }
}
