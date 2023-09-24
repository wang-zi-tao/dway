use anyhow::{bail, Result};
use bevy::prelude::*;
use drm::control::{connector, ModeTypeFlags};
use getset::Getters;

#[derive(Component,Clone, Debug, Getters)]
#[get="pub"]
pub struct Connector {
    pub(crate) info: connector::Info,
    pub(crate) name: String,
    pub(crate) size: IVec2,
    pub(crate) mode: drm::control::Mode,
}

impl Connector {
    #[tracing::instrument(skip_all)]
    pub fn new(info: connector::Info) -> Result<Self> {
        let modes = info.modes();
        if modes.len() == 0 {
            bail!("no display mode");
        }

        let mode = modes
            .iter()
            .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
            .cloned()
            .unwrap_or_else(|| modes[0]);

        let name = format!("{}-{}", info.interface().as_str(), info.interface_id());
        let size = info
            .size()
            .map(|(w, h)| IVec2::new(w as i32, h as i32))
            .unwrap_or_default();

        Ok(Self {
            info,
            name,
            size,
            mode,
        })
    }
}
