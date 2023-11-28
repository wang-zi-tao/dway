use std::path::PathBuf;

use bevy::{prelude::Resource, reflect::Reflect};
use clap::Parser;

#[derive(Parser, Debug, Resource, Clone, Reflect)]
#[command(author, version, about)]
pub struct DWayOption {
    /// output system graph
    #[arg(long)]
    pub debug_schedule: bool,
}

impl DWayOption {}
