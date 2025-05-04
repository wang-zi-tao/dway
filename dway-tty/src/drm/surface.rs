use std::{
    collections::{LinkedList, VecDeque},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, bail, Result};
use bevy::prelude::*;
use drm::{
    control::{
        atomic::AtomicModeReq,
        connector, crtc, plane,
        property::{self, Value},
        AtomicCommitFlags, Device, Mode, PageFlipEvent,
    },
    Device as drm_device, VblankWaitFlags,
};
use drm_fourcc::DrmFormat;
use measure_time::debug_time;
use tracing::{span, Level};
use wgpu::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::{connectors::Connector, planes::PlaneConfig, DrmDevice, DrmDeviceFd, PropMap};
use crate::{
    drm::{planes::Planes, DrmDeviceState},
    failure::DWayTTYError::*,
    gbm::buffer::GbmBuffer,
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
            DrmDeviceState::Atomic { props, .. } => Self::Atomic {
                props: props.clone(),
            },
            DrmDeviceState::Legacy { .. } => Self::Legacy {},
        };
        Ok(state)
    }
}

#[derive(Debug)]
pub struct SurfaceInner {
    pub(crate) state: SurfaceState,
    pub(crate) crtc: crtc::Handle,
    pub(crate) mode: Mode,
    pub(crate) mode_blob: Value<'static>,
    pub(crate) planes: Planes,
    pub(crate) transform: DrmTransform,
    pub(crate) formats: Vec<DrmFormat>,
    pub(crate) connector: connector::Handle,

    pub(crate) pedding: Option<GbmBuffer>,
    pub(crate) commited: LinkedList<GbmBuffer>,
    pub(crate) showing: Option<GbmBuffer>,
    pub(crate) available: VecDeque<GbmBuffer>,
}

impl SurfaceInner {
    pub fn buffer_count(&self) -> usize {
        self.pedding.iter().len() + self.commited.len() + self.available.len()
    }

    pub fn on_page_flip(&mut self, _event: &PageFlipEvent) {
        //if let Some(commited) = self.showing.take() {
        //    self.available.push_back(commited);
        //}
    }

    pub fn formats(&self) -> &[DrmFormat] {
        &self.formats
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
    pub fn new(drm: &DrmDevice, connector: &Connector, images: &mut Assets<Image>) -> Result<Self> {
        let crtc = drm.alloc_crtc(&connector.info)?;
        let mut planes = Planes::new(&crtc, drm)?;
        let plane_info = drm.get_plane(planes.primary.handle)?;

        let resources = drm.resource_handles()?;
        if !resources
            .filter_crtcs(plane_info.possible_crtcs())
            .contains(&crtc)
        {
            bail!(
                "cannot use {crtc:?} on {:?} on {:?}",
                planes.primary.handle,
                connector.info.handle()
            );
        }

        let crtc_data = drm.fd.get_crtc(crtc)?;
        let mode = crtc_data.mode().unwrap_or(connector.mode);

        let state = SurfaceState::new(drm)?;
        let size = mode.size();
        let image = images.add(create_image(IVec2::new(size.0 as i32, size.1 as i32)));
        let mode_blob = drm.create_property_blob(&mode)?;
        let formats = drm.formats(plane_info.handle())?;

        let driver = drm.get_driver()?;
        if driver
            .name()
            .to_string_lossy()
            .to_lowercase()
            .contains("nvidia")
            || driver
                .description()
                .to_string_lossy()
                .to_lowercase()
                .contains("nvidia")
        {
            planes.overlay = vec![];
        }

        Ok(Self {
            inner: Arc::new(Mutex::new(SurfaceInner {
                pedding: Default::default(),
                commited: Default::default(),
                available: Default::default(),

                state,
                crtc,
                mode,
                planes,
                formats: formats.into_iter().collect(),
                transform: DrmTransform::NORMAL,
                mode_blob,
                showing: None,
                connector: connector.info().handle(),
            })),
            image,
        })
    }

    pub fn size(&self) -> IVec2 {
        self.inner.lock().unwrap().size()
    }

    pub fn commit_buffer(&self, drm: &DrmDevice, buffer: &GbmBuffer) -> Result<()> {
        let self_guard = self.inner.lock().unwrap();
        let drm_guard = drm.inner.lock().unwrap();

        match (&self_guard.state, &drm_guard.states) {
            (
                SurfaceState::Atomic { .. },
                DrmDeviceState::Atomic {
                    props: drm_props, ..
                },
            ) => {
                let size = self_guard.size();
                let req = create_request(
                    &self_guard,
                    self_guard.connector,
                    &[(
                        self_guard.planes.primary.handle,
                        Some(PlaneConfig {
                            src: Rect::from_corners(Vec2::default(), size.as_vec2()),
                            dest: Rect::from_corners(Vec2::default(), size.as_vec2()),
                            transform: self_guard.transform,
                            framebuffer: buffer.framebuffer,
                        }),
                    )],
                    drm_props,
                )?;
                {
                    debug_time!("wait_vblank");
                    if let Err(e) = drm.wait_vblank(
                        drm::VblankWaitTarget::Relative(1),
                        VblankWaitFlags::NEXT_ON_MISS,
                        u32::from(self_guard.crtc) >> 27,
                        15,
                    ) {
                        error!("wait error: {e}");
                    };
                }
                {
                    let _span =
                        info_span!("atomic_commit",framebuffer=?buffer.framebuffer).entered();
                    debug_time!("atomic_commit");
                    drm.atomic_commit(
                        AtomicCommitFlags::ALLOW_MODESET | AtomicCommitFlags::NONBLOCK,
                        req.clone(),
                    )
                    .map_err(|e| anyhow!("failed to commit drm atomic request: {e}"))?;

                    drm.atomic_commit(
                        AtomicCommitFlags::ALLOW_MODESET | AtomicCommitFlags::NONBLOCK,
                        req,
                    )
                    .map_err(|e| anyhow!("failed to commit drm atomic request: {e}"))?; // TODO

                    debug!("commmit drm render buffer");
                }
            }
            (SurfaceState::Legacy {}, DrmDeviceState::Legacy { .. }) => todo!(),
            (SurfaceState::Atomic { .. }, DrmDeviceState::Legacy { .. }) => unreachable!(),
            (SurfaceState::Legacy {}, DrmDeviceState::Atomic { .. }) => unreachable!(),
        }

        Ok(())
    }

    pub fn image(&self) -> Handle<Image> {
        self.image.clone()
    }

    pub fn handle(&self) -> drm::control::connector::Handle {
        self.inner.lock().unwrap().connector
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
        ..default()
    });
    image
}

pub fn drm_framebuffer_descriptor<'l>(size: IVec2) -> TextureDescriptor<'l> {
    let image_size = Extent3d {
        width: size.x as u32,
        height: size.y as u32,
        ..default()
    };
    TextureDescriptor {
        label: Some("gbm_framebuffer"),
        size: image_size,
        dimension: TextureDimension::D2,
        format: TextureFormat::Bgra8UnormSrgb,
        mip_level_count: 1,
        sample_count: 1,
        usage: TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::COPY_DST
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    }
}

fn to_fixed<N: Into<f64> + Copy>(n: N) -> u64 {
    f64::round(n.into() * (1 << 16) as f64) as u64
}

pub fn create_request(
    surface: &SurfaceInner,
    conn: connector::Handle,
    planes: &[(plane::Handle, Option<PlaneConfig>)],
    drm_props: &PropMap,
) -> Result<AtomicModeReq> {
    use property::Value::*;

    let mut req = AtomicModeReq::new();

    req.add_property(
        conn,
        *drm_props
            .connector
            .get(&(conn, "CRTC_ID".to_string()))
            .ok_or_else(|| NoSuchProperty("CRTC_ID".to_string()))?,
        CRTC(Some(surface.crtc)),
    );

    req.add_property(
        surface.crtc,
        *drm_props
            .crtc
            .get(&(surface.crtc, "MODE_ID".to_string()))
            .ok_or_else(|| NoSuchProperty("MODE_ID".to_string()))?,
        surface.mode_blob,
    );

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
                UnsignedRange(to_fixed(config.src.min.x)),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_Y")?,
                UnsignedRange(to_fixed(config.src.min.y)),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_W")?,
                UnsignedRange(to_fixed(config.src.width())),
            );
            req.add_property(
                *plane_handle,
                plane_prop("SRC_H")?,
                UnsignedRange(to_fixed(config.src.height())),
            );

            req.add_property(
                *plane_handle,
                plane_prop("CRTC_X")?,
                Value::SignedRange(config.dest.min.x as i64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_Y")?,
                Value::SignedRange(config.dest.min.y as i64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_W")?,
                UnsignedRange(config.dest.width() as u64),
            );
            req.add_property(
                *plane_handle,
                plane_prop("CRTC_H")?,
                UnsignedRange(config.dest.height() as u64),
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

pub fn print_drm_info(drm: &DrmDeviceFd) -> Result<()> {
    let _span = span!(Level::INFO, "drm info");
    let res_handles = drm.resource_handles()?;
    for conn_handle in res_handles.connectors() {
        let Ok(conn) = drm.get_connector(*conn_handle, false) else {
            continue;
        };
        debug!("conn({:?})=>{:?}", conn_handle, conn);
    }
    for plane_handle in drm.plane_handles()? {
        let Ok(plane) = drm.get_plane(plane_handle) else {
            continue;
        };
        debug!("plane({:?})=>{:?}", plane_handle, plane);
    }
    for crtc_handle in res_handles.crtcs() {
        let Ok(crtc) = drm.get_crtc(*crtc_handle) else {
            continue;
        };
        debug!("crtc({:?})=>{:?}", crtc_handle, crtc);
    }
    for framebuffer_handle in res_handles.framebuffers() {
        let Ok(framebuffer) = drm.get_framebuffer(*framebuffer_handle) else {
            continue;
        };
        debug!("framebuffer({:?})=>{:?}", framebuffer_handle, framebuffer);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use bevy::prelude::*;
    use drm_fourcc::DrmFormat;
    use dway_util::eventloop::{EventLoopPlugin, EventLoopPluginMode};

    use super::DrmSurface;
    use crate::{
        drm::{DrmDevice, DrmPlugin},
        gbm::GbmDevice,
        schedule::DWayTtySchedulePlugin,
        seat::SeatPlugin,
        test::test_suite_plugins,
        udev::UDevPlugin,
    };

    #[test]
    pub fn test_create_drm_surface() {
        let mut app = App::new();
        app.add_plugins(test_suite_plugins());
        app.add_plugins((
            DWayTtySchedulePlugin,
            EventLoopPlugin {
                mode: EventLoopPluginMode::ManualMode,
                ..Default::default()
            },
            SeatPlugin,
            UDevPlugin {
                sub_system: "drm".into(),
            },
            DrmPlugin,
        ));
        app.add_systems(
            PostUpdate,
            |device_query: Query<(&DrmDevice, &GbmDevice)>,
             surface_query: Query<(&DrmSurface, &Parent)>| {
                for (surface, parent) in surface_query.iter() {
                    let (drm, gbm) = device_query.get(parent.get()).unwrap();
                    let surface_guard = surface.inner.lock().unwrap();
                    let formats = [DrmFormat {
                        code: drm_fourcc::DrmFourcc::Rgba8888,
                        modifier: gbm::Modifier::Linear,
                    }];
                    gbm.create_buffer(drm, surface_guard.size(), &formats, &formats)
                        .unwrap();
                }
            },
        );
        app.run();
    }
}
