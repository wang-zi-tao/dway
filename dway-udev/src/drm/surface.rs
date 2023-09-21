use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Context, Result};
use bevy::{
    ecs::storage::Resources,
    prelude::*,
    render::{renderer::RenderDevice, texture::GpuImage},
    utils::HashSet,
};
use drm::control::{
    atomic::AtomicModeReq, connector, crtc, plane, property, AtomicCommitFlags, Device, Mode,
};
use drm_ffi::drm_format_modifier_blob;
use drm_fourcc::{DrmFormat, DrmFourcc, DrmModifier};
use smallvec::SmallVec;
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use wgpu_hal::TextureUses;

use crate::{
    drm::{planes::Planes, DrmDeviceState},
    failure::DWayTTYError::*,
    gbm::{buffer::GbmBuffer, GbmDevice},
};

use super::{
    connectors::Connector, planes::PlaneConfig, DrmConnectorEvent, DrmDevice, DrmDeviceInner,
    PropMap,
};

bitflags::bitflags! {
    #[derive(Clone,Copy, Debug,Hash,PartialEq, Eq, PartialOrd, Ord)]
    pub struct DrmTransform: u8 {
        const ROTATE_0      =   0b00000001;
        const ROTATE_90     =   0b00000010;
        const ROTATE_180    =   0b00000100;
        const ROTATE_270    =   0b00001000;
        const REFLECT_X     =   0b00010000;
        const REFLECT_Y     =   0b00100000;

        const NORMAL = Self::ROTATE_0.bits();
    }
}
impl Default for DrmTransform {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Debug)]
pub enum SurfaceState {
    Atomic { props: PropMap },
    Legacy {},
}
impl SurfaceState {
    pub fn new(drm: &DrmDevice) -> Result<Self> {
        let guard = drm.inner.lock().unwrap();
        let state = match &guard.states {
            DrmDeviceState::Atomic { props, backup } => Self::Atomic {
                props: props.clone(),
            },
            DrmDeviceState::Legacy { backup } => Self::Legacy {},
        };
        Ok(state)
    }
}

#[derive(Debug)]
pub struct SurfaceInner {
    pub(crate) state: SurfaceState,
    pub(crate) crtc: crtc::Handle,
    pub(crate) mode: Mode,
    pub(crate) planes: Planes,
    pub(crate) transform: DrmTransform,
    pub(crate) formats: Vec<DrmFormat>,

    pub(crate) buffers: VecDeque<GbmBuffer>,
}

impl SurfaceInner {
    pub fn get_buffer(
        &mut self,
        drm: &DrmDevice,
        gbm: &GbmDevice,
        render_formats: &[DrmFormat],
    ) -> Result<&GbmBuffer> {
        if self.buffers.is_empty() {
            let size = self.mode.size();
            let size = IVec2::new(size.0 as i32, size.1 as i32);
            let gbm = gbm.create_buffer(drm, size, &*self.formats, render_formats)?;
            self.buffers.push_back(gbm);
        }
        Ok(self.buffers.front().unwrap())
    }

    pub fn with_rendering_buffer<R>(
        &mut self,
        drm: &DrmDevice,
        gbm: &GbmDevice,
        render_formats: &[DrmFormat],
        f: impl FnOnce(&GbmBuffer) -> Result<R>,
    ) -> Result<R> {
        if self.buffers.is_empty() {
            let size = self.mode.size();
            let size = IVec2::new(size.0 as i32, size.1 as i32);
            let gbm = gbm.create_buffer(drm, size, &*self.formats, render_formats)?;
            self.buffers.push_back(gbm);
        }
        f(&self.buffers.front().unwrap())
    }
}

impl SurfaceInner {
    pub fn size(&self) -> IVec2 {
        let size = self.mode.size();
        IVec2::new(size.0 as i32, size.1 as i32)
    }
}

#[derive(Component, Clone, Debug)]
pub struct DrmSurface {
    pub(crate) inner: Arc<Mutex<SurfaceInner>>,
    pub(crate) image: Handle<Image>,
}

impl DrmSurface {
    #[tracing::instrument(skip_all)]
    pub fn new(
        drm: &DrmDevice,
        connector: &Connector,
        crtc: crtc::Handle,
        images: &mut Assets<Image>,
    ) -> Result<Self> {
        let planes = Planes::new(&crtc, drm)?;
        let plane_info = drm.get_plane(planes.primary.handle)?;
        let crtcs = plane_info.possible_crtcs();
        // TODO check resource compatible

        let crtc_data = drm.fd.get_crtc(crtc)?;

        let state = SurfaceState::new(drm)?;
        let size = connector.mode.size();
        let image = images.add(create_image(IVec2::new(size.0 as i32, size.1 as i32)));

        let formats = drm.formats(plane_info.handle())?;

        Ok(Self {
            inner: Arc::new(Mutex::new(SurfaceInner {
                buffers: Default::default(),
                state,
                crtc,
                mode: connector.mode,
                planes,
                formats: formats.into_iter().collect(),
                transform: DrmTransform::NORMAL,
            })),
            image,
        })
    }

    pub fn size(&self) -> IVec2 {
        self.inner.lock().unwrap().size()
    }

    pub fn with_rendering_buffer<R>(
        &self,
        drm: &DrmDevice,
        gbm: &GbmDevice,
        render_formats: &[DrmFormat],
        f: impl FnOnce(&GbmBuffer) -> Result<R>,
    ) -> Result<R> {
        self.inner
            .lock()
            .unwrap()
            .with_rendering_buffer(drm, gbm, render_formats, f)
    }

    pub fn commit(&self, drm: &DrmDevice) -> Result<()> {
        let mut self_guard = self.inner.lock().unwrap();
        let mut drm_guard = drm.inner.lock().unwrap();
        let connector_change = drm_guard.connectors_change(&drm.fd)?;

        match (&self_guard.state, &drm_guard.states) {
            (
                SurfaceState::Atomic { props },
                DrmDeviceState::Atomic {
                    props: drm_props,
                    backup,
                },
            ) => {
                if let Some(buffer) = self_guard.buffers.front() {
                    let size = self_guard.size();
                    let req = create_request(
                        &self_guard,
                        connector_change,
                        &[(
                            self_guard.planes.primary.handle,
                            Some(PlaneConfig {
                                src: Rect::from_center_size(Vec2::default(), size.as_vec2()),
                                dest: Rect::from_center_size(Vec2::default(), size.as_vec2()),
                                transform: self_guard.transform,
                                framebuffer: buffer.framebuffer,
                            }),
                        )],
                        drm_props,
                    )?; // TODO

                    drm.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, req)
                        .map_err(|e| anyhow!("failed to commit drm atomic req: {e}"))?;
                }
            }
            (SurfaceState::Legacy {}, DrmDeviceState::Legacy { backup }) => todo!(),
            (SurfaceState::Atomic { props }, DrmDeviceState::Legacy { backup }) => unreachable!(),
            (SurfaceState::Legacy {}, DrmDeviceState::Atomic { props, backup }) => unreachable!(),
        }

        Ok(())
    }
    pub fn bind(&mut self, device: &mut RenderDevice) -> Result<GpuImage> {
        todo!()
    }
}

pub fn create_image(size: IVec2) -> Image {
    let mut image = Image {
        texture_descriptor: drm_framebuffer_descriptor(size),
        ..default()
    };
    image.resize(Extent3d {
        width: size.x as u32,
        height: size.y as u32,
        depth_or_array_layers: 1,
        ..default()
    });
    image
}

pub fn drm_framebuffer_descriptor<'l>(size: IVec2) -> TextureDescriptor<'l> {
    let image_size = Extent3d {
        width: size.x as u32,
        height: size.y as u32,
        depth_or_array_layers: 1,
        ..default()
    };
    TextureDescriptor {
        label: Some("gbm framebuffer"),
        size: image_size,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    }
}

fn to_fixed<N: Into<f64>>(n: N) -> u32 {
    f64::round(n.into() * (1 << 16) as f64) as u32
}

pub fn create_request(
    surface: &SurfaceInner,
    connector_change: SmallVec<[DrmConnectorEvent; 1]>,
    planes: &[(plane::Handle, Option<PlaneConfig>)],
    drm_props: &PropMap,
) -> Result<AtomicModeReq> {
    use property::Value::*;

    let buffer = surface.buffers.get(0).ok_or_else(|| anyhow!("no buffer"))?;

    let mut req = AtomicModeReq::new();

    for change in connector_change {
        match change {
            super::DrmConnectorEvent::Added(connector) => {
                req.add_property(
                    connector.handle(),
                    *drm_props
                        .connector
                        .get(&(connector.handle(), "CRTC_ID".to_string()))
                        .ok_or_else(|| NoSuchProperty("CRTC_ID".to_string()))?,
                    CRTC(Some(surface.crtc)),
                );
            }
            super::DrmConnectorEvent::Removed(connector, _) => {
                req.add_property(
                    connector.handle(),
                    *drm_props
                        .connector
                        .get(&(connector.handle(), "CRTC_ID".to_string()))
                        .ok_or_else(|| NoSuchProperty("CRTC_ID".to_string()))?,
                    CRTC(None),
                );
            }
        }
    }

    if let Some(blob) = buffer.blob {
        req.add_property(
            surface.crtc,
            *drm_props
                .crtc
                .get(&(surface.crtc, "MODE_ID".to_string()))
                .ok_or_else(|| NoSuchProperty("MODE_ID".to_string()))?,
            blob,
        );
    }

    req.add_property(
        surface.crtc,
        *drm_props
            .crtc
            .get(&(surface.crtc, "ACTIVE".to_string()))
            .ok_or_else(|| NoSuchProperty("ACTIVE".to_string()))?,
        Boolean(true),
    );

    for (plane_handle, config) in planes {
        let plane_prop = |key: &str| {
            drm_props
                .plane
                .get(&(*plane_handle, key.to_string()))
                .ok_or_else(|| NoSuchProperty(key.to_string()))
                .cloned()
        };

        if let Some(config) = config {
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_ID")?,
                CRTC(Some(surface.crtc)),
            );
            req.add_property(
                *plane_handle,
                plane_prop("FB_ID")?,
                Framebuffer(Some(config.framebuffer)),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_X")?,
                UnsignedRange(to_fixed(config.src.min.x) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_Y")?,
                UnsignedRange(to_fixed(config.src.min.y) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_W")?,
                UnsignedRange(to_fixed(config.src.width()) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_H")?,
                UnsignedRange(to_fixed(config.src.height()) as u64),
            );

            req.add_property(
                *plane_handle,
                plane_prop("CRTC_X")?,
                UnsignedRange(to_fixed(config.dest.min.x) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_Y")?,
                UnsignedRange(to_fixed(config.dest.min.y) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_W")?,
                UnsignedRange(to_fixed(config.dest.width()) as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_H")?,
                UnsignedRange(to_fixed(config.dest.height()) as u64),
            );

            if let Some(prop) = drm_props
                .plane
                .get(&(*plane_handle, "rotation".to_string()))
            {
                req.add_property(
                    *plane_handle,
                    *prop,
                    Bitmask(config.transform.bits() as u64),
                )
            }
        } else {
            req.add_property(*plane_handle, plane_prop("CRTC_ID")?, CRTC(None));
            req.add_property(*plane_handle, plane_prop("FB_ID")?, Framebuffer(None));
            req.add_property(*plane_handle, plane_prop("SRC_X")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("SRC_Y")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("SRC_W")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("SRC_H")?, UnsignedRange(0));

            req.add_property(*plane_handle, plane_prop("CRTC_X")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("CRTC_Y")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("CRTC_W")?, UnsignedRange(0));
            req.add_property(*plane_handle, plane_prop("CRTC_H")?, UnsignedRange(0));

            if let Some(prop) = drm_props
                .plane
                .get(&(*plane_handle, "rotation".to_string()))
            {
                req.add_property(
                    *plane_handle,
                    *prop,
                    Bitmask(DrmTransform::NORMAL.bits() as u64),
                )
            }
        }
    }

    Ok(req)
}
