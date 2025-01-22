use std::{collections::VecDeque, os::fd::OwnedFd};

use wayland_backend::server::ClientId;

use crate::prelude::*;

pub struct ClipboardRecord {
    pub mime_type: String,
    pub fd: OwnedFd,
    pub client: ClientId,
}

#[derive(Resource)]
pub struct ClipboardManager {
    pub records: VecDeque<ClipboardRecord>,
}

impl ClipboardManager {
    pub fn push(&mut self, recoed: ClipboardRecord) {
        self.records.push_back(recoed);
    }

    pub fn get(&self) -> Option<&ClipboardRecord> {
        self.records.back()
    }
}
