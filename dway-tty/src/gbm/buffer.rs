use std::{os::fd::OwnedFd, sync::Mutex};

use anyhow::Result;
use bevy::prelude::IVec2;
use drm::control::Device;
use drm_fourcc::{DrmFourcc, DrmModifier};
use getset::Getters;
use smallvec::SmallVec;
use tracing::{debug, error};

use crate::drm::{DrmDevice, DrmDeviceFd};

bitflags::bitflags! {
    pub struct DmabufFlags: u32 {
        const Y_INVERT = 1;
        const INTERLACED = 2;
        const BOTTOM_FIRST = 4;
    }
}

#[derive(Debug)]
pub struct Plane {
    pub fd: OwnedFd,
    pub offset: u32,
    pub stride: u32,
}

#[derive(Debug, Getters)]
#[get = "pub"]
pub struct GbmBuffer {
    pub(crate) drm: DrmDeviceFd,
    pub(crate) framebuffer: drm::control::framebuffer::Handle,
    pub(crate) buffer: Mutex<gbm::BufferObject<()>>,
    pub(crate) planes: SmallVec<[Plane; 4]>,
    pub(crate) size: IVec2,
    pub(crate) format: DrmFourcc,
    pub(crate) modifier: DrmModifier,
}

impl GbmBuffer {
    pub fn new(
        drm: &DrmDevice,
        buffer: gbm::BufferObject<()>,
        size: IVec2,
        format: DrmFourcc,
    ) -> Result<Self> {
        let planes_count = buffer.plane_count()?;
        let mut planes = SmallVec::default();

        for plane_number in 0..planes_count as i32 {
            planes.push(Plane {
                fd: buffer.fd_for_plane(plane_number)?,
                offset: buffer.offset(plane_number)?,
                stride: buffer.stride_for_plane(plane_number)?,
            });
        }

        let framebuffer = drm.create_framebuffer(&buffer)?;

        Ok(Self {
            modifier: buffer.modifier()?,
            planes,
            size,
            format,
            buffer: Mutex::new(buffer),
            drm: drm.fd.clone(),
            framebuffer,
        })
    }
}

impl Drop for GbmBuffer {
    fn drop(&mut self) {
        debug!("destroy gbm buffer");
        if let Err(e) = self.drm.destroy_framebuffer(self.framebuffer) {
            error!("failed to destroy drm framebuffer: {e}");
        };
    }
}
