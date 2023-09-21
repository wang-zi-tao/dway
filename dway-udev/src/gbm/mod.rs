pub mod buffer;

use anyhow::anyhow;
use anyhow::{bail, Result};
use bevy::prelude::*;
use bevy::utils::HashSet;
use drm::control::{connector, crtc, from_u32, plane, Device, Mode};
use drm_fourcc::DrmModifier;
use drm_fourcc::{DrmFormat, DrmFourcc};
use gbm::BufferObjectFlags;
use gbm::Format;
use libseat::Seat;
use std::sync::Arc;
use std::{
    os::fd::{AsFd, AsRawFd, OwnedFd, RawFd},
    path::{Path, PathBuf},
    sync::Mutex,
};

use crate::{
    drm::{planes::Planes, DrmDevice, DrmDeviceFd},
    seat::DeviceFd,
    util::transmute_vec_from_u32,
};

const SUPPORTED_FORMATS: [DrmFourcc; 1] = [DrmFourcc::Argb8888];

use self::buffer::GbmBuffer;

#[derive(Component, Clone)]
pub struct GbmDevice {
    pub(crate) device: Arc<Mutex<gbm::Device<DrmDeviceFd>>>,
}
impl GbmDevice {
    #[tracing::instrument(skip_all)]
    pub fn new(device: DrmDeviceFd) -> Result<Self> {
        Ok(Self {
            device: Arc::new(Mutex::new(gbm::Device::new(device)?)),
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn create_buffer(
        &self,
        drm: &DrmDevice,
        size: IVec2,
        drm_formats: &[DrmFormat],
        render_formats: &[DrmFormat],
    ) -> Result<GbmBuffer> {
        let guard = self.device.lock().unwrap();

        let focurcc_set =
            HashSet::from_iter(drm_formats.iter().chain(render_formats).map(|f| f.code));
        let (buffer, format) = SUPPORTED_FORMATS
            .iter()
            .find_map(|fourcc| {
                let drm_formats = drm_formats
                    .iter()
                    .filter(|f| f.code == *fourcc)
                    .collect::<HashSet<_>>();
                let render_formats = render_formats
                    .iter()
                    .filter(|f| f.code == *fourcc)
                    .collect::<HashSet<_>>();
                let modifiers = drm_formats
                    .intersection(&render_formats)
                    .map(|f| f.modifier)
                    .collect::<Vec<_>>();

                guard
                    .create_buffer_object_with_modifiers2(
                        size.x as u32,
                        size.y as u32,
                        *fourcc,
                        modifiers.iter().cloned(),
                        BufferObjectFlags::RENDERING | BufferObjectFlags::SCANOUT,
                    )
                    .map_err(
                        |e| info!(?fourcc,modifiers=?modifiers,"try to create gbm buffer: {e}"),
                    )
                    .or_else(|_| {
                        guard.create_buffer_object_with_modifiers2(
                            size.x as u32,
                            size.y as u32,
                            *fourcc,
                            drm_formats.iter().map(|f| f.modifier),
                            BufferObjectFlags::RENDERING | BufferObjectFlags::SCANOUT,
                        )
                    })
                    .map_err(|e| info!(format=?drm_formats,"try to create gbm buffer: {e}"))
                    .or_else(|_| {
                        guard.create_buffer_object_with_modifiers2(
                            size.x as u32,
                            size.y as u32,
                            *fourcc,
                            [
                                DrmModifier::Linear,
                                DrmModifier::I915_x_tiled,
                                DrmModifier::I915_y_tiled,
                                DrmModifier::I915_y_tiled_gen12_rc_ccs,
                            ]
                            .iter()
                            .cloned(),
                            BufferObjectFlags::RENDERING | BufferObjectFlags::SCANOUT,
                        )
                    })
                    .map_err(|e| warn!(?fourcc, "try to create gbm buffer: {e}"))
                    .ok()
                    .map(|b| (b, *fourcc))
            })
            .ok_or_else(|| anyhow!("no supported format"))?;
        Ok(GbmBuffer::new(drm, buffer, size, format)?)
    }
}
