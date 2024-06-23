

use bevy::{prelude::Resource, reflect::Reflect};
use clap::Parser;

#[derive(Parser, Debug, Resource, Clone, Reflect)]
#[command(author, version, about)]
pub struct DWayOption {
    /// output system graph
    #[arg(long)]
    pub debug_schedule: bool,
    #[arg(long, default_value_t = 60.0)]
    pub frame_rate: f32,
}

impl DWayOption {}
