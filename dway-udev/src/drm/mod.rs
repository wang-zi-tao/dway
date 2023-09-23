pub mod connectors;
pub mod planes;
pub mod surface;
pub mod util;

use crate::drm::planes::Planes;
use crate::drm::surface::DrmSurface;
use crate::failure::DWayTTYError::*;
use crate::gbm::buffer::GbmBuffer;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use bevy::prelude::*;
use bevy::render::render_phase::RenderCommandResult;
use bevy::utils::hashbrown::hash_map::Entry;
use bevy::utils::tracing::instrument;
use bevy::utils::HashMap;
use bevy::{
    render::{renderer::RenderDevice, Extract},
    utils::tracing,
};
use bitflags::bitflags;
use double_map::DHashMap;
use drm::control::atomic::AtomicModeReq;
use drm::control::connector;
use drm::control::crtc;
use drm::control::framebuffer;
use drm::control::plane;
use drm::control::property;
use drm::control::AtomicCommitFlags;
use drm::control::Device as DrmControlDevice;
use drm::control::Mode;
use drm::control::PageFlipEvent;
use drm::control::PropertyValueSet;
use drm::control::RawResourceHandle;
use drm::control::ResourceHandle;
use drm::control::VblankEvent;
use drm::Device;
use drm::Driver;
use drm::SystemError;
use drm_ffi::drm_format_modifier_blob;
use drm_ffi::DRM_MODE_FB_MODIFIERS;
use drm_fourcc::DrmFormat;
use drm_fourcc::DrmFourcc;
use drm_fourcc::DrmModifier;
use gbm::BufferObject;
use libseat::Seat;
use nix::libc;
use scopeguard::defer;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::io;
use std::os::fd::AsFd;
use std::path::Path;
use std::path::PathBuf;
use std::{
    any,
    collections::{BTreeSet, HashSet},
    ffi::{c_char, CStr, CString, OsString},
    fs::File,
    ptr::null,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};
use tracing::span;
use tracing::Level;
use udev::Enumerator;

use crate::gbm::GbmDevice;
use crate::schedule::DWayTTYSet;
use crate::seat::DeviceFd;
use crate::seat::SeatState;
use crate::udev::UDevEvent;
use crate::udev::UDevMonitor;

use self::connectors::Connector;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrmNodeType {
    Primary,
    Control,
    Render,
}
#[derive(Debug, Clone)]
pub struct DrmNode {
    pub device: libc::dev_t,
    pub kind: DrmNodeType,
}
impl DrmNode {
    pub fn new(path: &Path) -> Result<Self> {
        let dev_stat =
            nix::sys::stat::stat(path).map_err(|e| anyhow!("failed to get file stat: {}", e))?;
        let dev = dev_stat.st_rdev;

        let major = ((dev >> 32) & 0xffff_f000) | ((dev >> 8) & 0x0000_0fff);
        let minor = ((dev >> 12) & 0xffff_ff00) | ((dev) & 0x0000_00ff);

        let path = format!("/sys/dev/char/{}:{}/device/drm", major, minor);
        nix::sys::stat::stat(path.as_str())
            .map_err(|e| anyhow!("failed to get file stat: {}", e))?;
        let ty = match minor >> 6 {
            0 => DrmNodeType::Primary,
            1 => DrmNodeType::Control,
            2 => DrmNodeType::Render,
            _ => return Err(anyhow!("not a drm node")),
        };
        Ok(Self {
            device: dev,
            kind: ty,
        })
    }
}

#[derive(Clone, Debug)]
pub struct DrmDeviceFd(pub DeviceFd);
impl AsFd for DrmDeviceFd {
    fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl drm::Device for DrmDeviceFd {}
impl drm::control::Device for DrmDeviceFd {}
impl DrmDeviceFd {
    pub fn try_with_prop<R, F: FnOnce(property::Info, u64) -> Result<R>, T: ResourceHandle>(
        &self,
        handle: T,
        name: &str,
        f: F,
    ) -> Result<Option<R>> {
        let props = self.get_properties(handle)?;
        let (ids, vals) = props.as_props_and_values();
        for (&id, &value) in ids.iter().zip(vals.iter()) {
            let prop_info = self.get_property(id)?;
            if prop_info
                .name()
                .to_str()
                .map(|n| n == name)
                .unwrap_or(false)
            {
                return f(prop_info, value).map(|v| Some(v));
            }
        }
        Ok(None)
    }

    pub fn with_prop<R, F: FnOnce(property::Info, u64) -> Result<R>, T: ResourceHandle>(
        &self,
        handle: T,
        name: &str,
        f: F,
    ) -> Result<R> {
        self.try_with_prop(handle, name, f)?
            .ok_or_else(|| NoSuchProperty(name.into()).into())
    }

    pub fn formats(&self, plane: plane::Handle) -> Result<HashSet<DrmFormat>> {
        let mut formats = HashSet::new();
        for format in self.get_plane(plane)?.formats() {
            formats.insert(DrmFormat {
                code: DrmFourcc::try_from(*format)?,
                modifier: DrmModifier::Invalid,
            });
        }
        self.try_with_prop(plane, "IN_FORMATS", |prop, value| {
            if let property::Value::Blob(blob) = prop.value_type().convert_value(value) {
                let data = self.get_property_blob(blob)?;
                unsafe {
                    let fmt_mod_blob_ptr = data.as_ptr() as *const drm_format_modifier_blob;
                    let fmt_mod_blob = &*fmt_mod_blob_ptr;

                    let formats_ptr: *const u32 = fmt_mod_blob_ptr
                        .cast::<u8>()
                        .offset(fmt_mod_blob.formats_offset as isize)
                        as *const _;
                    let modifiers_ptr: *const drm_ffi::drm_format_modifier = fmt_mod_blob_ptr
                        .cast::<u8>()
                        .offset(fmt_mod_blob.modifiers_offset as isize)
                        as *const _;
                    let formats_ptr = formats_ptr as *const u32;
                    let modifiers_ptr = modifiers_ptr as *const drm_ffi::drm_format_modifier;

                    for i in 0..fmt_mod_blob.count_modifiers {
                        let mod_info = modifiers_ptr.offset(i as isize).read_unaligned();
                        for j in 0..64 {
                            if mod_info.formats & (1u64 << j) != 0 {
                                let code = DrmFourcc::try_from(
                                    formats_ptr
                                        .offset((j + mod_info.offset) as isize)
                                        .read_unaligned(),
                                )
                                .ok();
                                let modifier = DrmModifier::from(mod_info.modifier);
                                if let Some(code) = code {
                                    formats.insert(DrmFormat { code, modifier });
                                }
                            }
                        }
                    }
                }
            }
            if formats.is_empty() {
                formats.insert(DrmFormat {
                    code: DrmFourcc::Argb8888,
                    modifier: DrmModifier::Invalid,
                });
            }
            Ok(())
        })?;
        Ok(formats)
    }
}

#[derive(Default, Debug)]
pub struct PropBackup {
    pub connector: HashMap<connector::Handle, PropertyValueSet>,
    pub crtc: HashMap<crtc::Handle, PropertyValueSet>,
    pub framebuffer: HashMap<framebuffer::Handle, PropertyValueSet>,
    pub plane: HashMap<plane::Handle, PropertyValueSet>,
}
#[derive(Default, Debug, Clone)]
pub struct PropMap {
    pub connector: HashMap<(connector::Handle, String), property::Handle>,
    pub crtc: HashMap<(crtc::Handle, String), property::Handle>,
    pub framebuffer: HashMap<(framebuffer::Handle, String), property::Handle>,
    pub plane: HashMap<(plane::Handle, String), property::Handle>,
}

pub enum DrmDeviceState {
    Atomic {
        props: PropMap,
        backup: PropBackup,
    },
    Legacy {
        backup: HashMap<crtc::Handle, (crtc::Info, Vec<connector::Handle>)>,
    },
}
pub fn do_backup<H: ResourceHandle + std::hash::Hash + Eq>(
    fd: &DrmDeviceFd,
    handles: &[H],
    map: &mut HashMap<H, PropertyValueSet>,
) -> Result<()> {
    map.clear();
    for handle in handles {
        let props = fd.get_properties(*handle).map_err(GetPropertyError)?;
        map.insert(*handle, props);
    }
    Ok(())
}
pub fn do_dump_props<H: ResourceHandle + std::hash::Hash + Eq>(
    fd: &DrmDeviceFd,
    handles: &[H],
    map: &mut HashMap<(H, String), property::Handle>,
) -> Result<()> {
    map.clear();
    for handle in handles {
        let props = fd.get_properties(*handle).map_err(GetPropertyError)?;
        for prop in props.as_props_and_values().0 {
            if let Ok(info) = fd.get_property(*prop) {
                let name = info.name().to_string_lossy().into_owned();
                map.insert((*handle, name), *prop);
            }
        }
    }
    Ok(())
}
pub fn set_connector_state(
    dev: &DrmDeviceFd,
    connectors: impl Iterator<Item = connector::Handle>,
    enabled: bool,
) -> Result<()> {
    for conn in connectors {
        let info = dev.get_connector(conn, false)?;
        if info.state() == connector::State::Connected {
            let props = dev.get_properties(conn).map_err(GetPropertyError)?;
            let (handles, _) = props.as_props_and_values();
            for handle in handles {
                let info = dev.get_property(*handle).map_err(GetPropertyError)?;
                if info.name().to_str().map(|x| x == "DPMS").unwrap_or(false) {
                    trace!(connector = ?conn, "Setting DPMS {}", enabled);
                    dev.set_property(
                        conn,
                        *handle,
                        if enabled {
                            0 /*DRM_MODE_DPMS_ON*/
                        } else {
                            3 /*DRM_MODE_DPMS_OFF*/
                        },
                    )
                    .map_err(SetPropertyError)?;
                }
            }
        }
    }
    Ok(())
}
impl DrmDeviceState {
    pub fn new(fd: &DrmDeviceFd) -> Result<Self> {
        let res_handles = fd.resource_handles().map_err(ResourceHandlesError)?;
        if fd
            .set_client_capability(drm::ClientCapability::Atomic, true)
            .is_ok()
        {
            debug!("drm backuping in atomic mode");
            let planes = fd.plane_handles().map_err(PlanesHandlesError)?;
            let mut backup = PropBackup::default();
            let mut props = PropMap::default();

            do_backup(fd, res_handles.connectors(), &mut backup.connector)?;
            do_backup(fd, res_handles.crtcs(), &mut backup.crtc)?;
            do_backup(fd, res_handles.framebuffers(), &mut backup.framebuffer)?;
            do_backup(fd, &planes, &mut backup.plane)?;

            do_dump_props(fd, res_handles.connectors(), &mut props.connector)?;
            do_dump_props(fd, res_handles.crtcs(), &mut props.crtc)?;
            do_dump_props(fd, &planes, &mut props.plane)?;

            Ok(Self::Atomic { props, backup })
        } else {
            debug!("drm backuping in legacy mode");
            let mut backup = HashMap::default();
            for conn_handle in res_handles.connectors() {
                let connector = fd
                    .get_connector(*conn_handle, false)
                    .map_err(GetConnectorError)?;
                let Some(enc_handle) = connector.current_encoder() else {
                    continue;
                };
                let enc = fd.get_encoder(enc_handle).map_err(GetEncoderError)?;
                let Some(crtc_handle) = enc.crtc() else {
                    continue;
                };
                let crtc = fd.get_crtc(crtc_handle).map_err(GetCrtcError)?;

                backup
                    .entry(crtc_handle)
                    .or_insert_with(|| (crtc, Vec::new()))
                    .1
                    .push(*conn_handle);
            }
            Ok(Self::Legacy { backup })
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn reset(&mut self, fd: &DrmDeviceFd) -> Result<()> {
        match self {
            DrmDeviceState::Atomic { props, backup } => {
                let res_handles = fd.resource_handles().map_err(ResourceHandlesError)?;
                let planes = fd.plane_handles().map_err(PlanesHandlesError)?;
                let mut req = AtomicModeReq::new();

                for handle in res_handles.connectors() {
                    let prop = props
                        .connector
                        .get(&(*handle, "CRTC_ID".to_owned()))
                        .ok_or_else(|| anyhow!("Unknown handle"))?;
                    req.add_property(*handle, *prop, property::Value::CRTC(None));
                }

                for handle in planes {
                    let prop = props
                        .plane
                        .get(&(handle, "CRTC_ID".to_owned()))
                        .ok_or_else(|| anyhow!("Unknown handle"))?;
                    req.add_property(handle, *prop, property::Value::CRTC(None));
                    let prop = props
                        .plane
                        .get(&(handle, "FB_ID".to_owned()))
                        .ok_or_else(|| anyhow!("Unknown handle"))?;
                    req.add_property(handle, *prop, property::Value::Framebuffer(None));
                }

                for handle in res_handles.crtcs() {
                    let mode_prop = props
                        .crtc
                        .get(&(*handle, "MODE_ID".to_owned()))
                        .ok_or_else(|| anyhow!("Unknown handle"))?;
                    let active_prop = props
                        .crtc
                        .get(&(*handle, "ACTIVE".to_owned()))
                        .ok_or_else(|| anyhow!("Unknown handle"))?;
                    req.add_property(*handle, *active_prop, property::Value::Boolean(false));
                    req.add_property(*handle, *mode_prop, property::Value::Unknown(0));
                }

                fd.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, req)
                    .map_err(AtomicCommitError)?;
            }
            DrmDeviceState::Legacy { backup } => {
                let res_handles = fd.resource_handles().map_err(ResourceHandlesError)?;
                set_connector_state(&fd, res_handles.connectors().iter().copied(), false)?;

                for crtc in res_handles.crtcs() {
                    #[allow(deprecated)]
                    fd.set_cursor(*crtc, Option::<&drm::control::dumbbuffer::DumbBuffer>::None)
                        .map_err(SetCursorStateError)?;
                    fd.set_crtc(*crtc, None, (0, 0), &[], None)
                        .map_err(SetCrtcStateError)?;
                }
            }
        }
        Ok(())
    }
}

pub struct DrmDeviceInner {
    pub(crate) privileged: bool,
    connectors: HashMap<connector::Handle, (Option<Entity>, connector::Info)>,
    pub(crate) enabled: bool,
    pub(crate) states: DrmDeviceState,
    pub(crate) driver: Driver,

    pub(crate) has_universal_planes: bool,
    pub(crate) connector_crtc_map: DHashMap<connector::Handle, crtc::Handle, ()>,
}

impl DrmDeviceInner {
    pub fn connectors_change(
        &mut self,
        fd: &DrmDeviceFd,
    ) -> Result<SmallVec<[DrmConnectorEvent; 1]>> {
        let res_handles = fd.resource_handles()?;
        let connector_handles = res_handles.connectors();
        let mut events = SmallVec::default();

        for conn in connector_handles
            .iter()
            .filter_map(|conn| fd.get_connector(*conn, true).ok())
        {
            use connector::State::*;
            if let Some((entity, ref mut old_conn)) = self.connectors.get_mut(&conn.handle()) {
                match (old_conn.state(), conn.state()) {
                    (Connected, Connected) => {}
                    (Connected, Disconnected) => {
                        events.push(DrmConnectorEvent::Removed(conn.clone(), *entity))
                    }
                    (Disconnected, Connected) => {
                        events.push(DrmConnectorEvent::Added(conn.clone()))
                    }
                    (Disconnected, Disconnected) => {}
                    (Unknown, _) => {}
                    (_, Unknown) => {}
                }
                *old_conn = conn;
            } else {
                events.push(DrmConnectorEvent::Added(conn));
            }
        }
        Ok(events)
    }
}

#[derive(Component, Clone)]
pub struct DrmDevice {
    pub(crate) fd: DrmDeviceFd,
    pub(crate) path: PathBuf,
    pub(crate) inner: Arc<Mutex<DrmDeviceInner>>,
}

impl std::ops::Deref for DrmDevice {
    type Target = DrmDeviceFd;

    fn deref(&self) -> &Self::Target {
        &self.fd
    }
}
pub enum DrmConnectorEvent {
    Added(connector::Info),
    Removed(connector::Info, Option<Entity>),
}
impl DrmDevice {
    #[tracing::instrument(skip_all)]
    pub fn new(device: DeviceFd, path: PathBuf) -> Result<Self> {
        let fd = DrmDeviceFd(device);
        let mut states = DrmDeviceState::new(&fd)?;
        states.reset(&fd)?;

        let has_universal_planes = fd
            .set_client_capability(drm::ClientCapability::UniversalPlanes, true)
            .is_ok();

        let privileged = fd.acquire_master_lock().is_ok();
        if !privileged {
            warn!("failed to acquire master lock");
        };

        let driver = fd.get_driver()?;
        info!("driver: {driver:?}");

        Ok(Self {
            fd,
            path,
            inner: Arc::new(Mutex::new(DrmDeviceInner {
                has_universal_planes,
                privileged,
                connectors: Default::default(),
                enabled: true,
                states,
                connector_crtc_map: Default::default(),
                driver,
            })),
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn connectors(&self) -> Result<Vec<Connector>> {
        let res_handles = self.fd.resource_handles().map_err(ResourceHandlesError)?;
        let raw_connectors = res_handles.connectors();

        let connectors = raw_connectors
            .iter()
            .filter_map(|conn| {
                self.fd
                    .get_connector(*conn, true)
                    .map_err(GetConnectorError)
                    .ok()
            })
            .filter(|conn| conn.state() == connector::State::Connected)
            .map(|conn| Connector::new(conn))
            .try_collect()?;
        Ok(connectors)
    }

    pub fn connectors_change(&self) -> Result<SmallVec<[DrmConnectorEvent; 1]>> {
        self.inner.lock().unwrap().connectors_change(&self.fd)
    }

    pub fn create_framebuffer(&self, buffer: &BufferObject<()>) -> Result<framebuffer::Handle> {
        let modifier = match buffer.modifier()? {
            DrmModifier::Invalid => None,
            x => Some(x),
        };
        let plane_count = buffer.plane_count()?;
        let handle = if let Some(modifier) = modifier {
            let modifiers = [
                Some(modifier),
                (plane_count > 1).then_some(modifier),
                (plane_count > 2).then_some(modifier),
                (plane_count > 3).then_some(modifier),
            ];
            self.add_planar_framebuffer(buffer, &modifiers, DRM_MODE_FB_MODIFIERS)
        } else {
            let modifiers = [None, None, None, None];
            self.add_planar_framebuffer(buffer, &modifiers, DRM_MODE_FB_MODIFIERS)
        };
        let handle = handle.or_else(|_| {
            if plane_count > 1 {
                bail!("too many plane");
            }
            Ok(self.add_framebuffer(buffer, 32, 32)?) // TODO
        })?;
        Ok(handle)
    }

    pub fn alloc_crtc(&self, connector: &connector::Info) -> Result<crtc::Handle> {
        if let Some(res_handles) = connector
            .current_encoder()
            .and_then(|e| self.get_encoder(e).ok())
            .and_then(|e| e.crtc())
        {
            return Ok(res_handles);
        };

        let mut guard = self.inner.lock().unwrap();
        let res_handles = self.fd.resource_handles()?;
        let crtc_handle = connector
            .encoders()
            .iter()
            .flat_map(|h| self.fd.get_encoder(*h))
            .find_map(|encoder| {
                res_handles
                    .filter_crtcs(encoder.possible_crtcs())
                    .into_iter()
                    .find(|crtc| guard.connector_crtc_map.get_key2(crtc).is_none())
            })
            .ok_or_else(|| anyhow!("no avalible crtc"))?;

        guard
            .connector_crtc_map
            .insert(connector.handle(), crtc_handle, ());
        Ok(crtc_handle)
    }
}

#[tracing::instrument(skip_all)]
pub fn all_gpus(seat: &SeatState) -> io::Result<Vec<PathBuf>> {
    let mut enumerator = Enumerator::new()?;
    enumerator.match_subsystem("drm")?;
    enumerator.match_sysname("card[0-9]*")?;
    Ok(enumerator
        .scan_devices()?
        .filter(|device| {
            &device
                .property_value("ID_SEAT")
                .map(|x| x.to_string_lossy())
                .unwrap_or_else(|| Cow::from("seat0"))
                == &seat.name
        })
        .flat_map(|device| device.devnode().map(PathBuf::from))
        .collect())
}

#[tracing::instrument(skip_all)]
pub fn setup(
    mut udev: NonSendMut<UDevMonitor>,
    mut seat: NonSendMut<SeatState>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    debug!(r"DRM Debugging:
    echo 0x19F | sudo tee /sys/module/drm/parameters/debug
    sudo dmesg -C
    dmesg -w
    ");

    for gpu_path in all_gpus(&seat).unwrap() {
        if let Err(e) = add_device(gpu_path, &mut udev, &mut seat, &mut commands, &mut images) {
            error!("failed to add drm device: {e}");
            info!("{}", e.backtrace());
        }
    }
}

pub fn add_device(
    gpu_path: PathBuf,
    udev: &mut UDevMonitor,
    seat: &mut SeatState,
    commands: &mut Commands,
    images: &mut Assets<Image>,
) -> Result<Entity> {
    let _span = span!(Level::ERROR,"init drm device",path=%gpu_path.to_string_lossy()).entered();

    let drm = seat
        .open_device(&gpu_path)
        .and_then(|fd| DrmDevice::new(fd, gpu_path.clone()))?;
    let gbm = GbmDevice::new(drm.fd.clone())?;

    let connectors = drm.connectors()?;

    let drm_entity = commands.spawn_empty().id();

    {
        for conn in connectors {
            trace!("conn: {:?}", conn);
            let mut entity_mut = commands.spawn_empty();

            let mut guard = drm.inner.lock().unwrap();
            guard
                .connectors
                .get_mut(&conn.info.handle())
                .map(|v| v.0 = Some(entity_mut.id()));

            drop(guard);
            let surface = DrmSurface::new(&drm, &conn, images)?;

            trace!("drm surface: {:?}", &surface);
            entity_mut.insert(surface);

            let name = conn.name.clone();
            entity_mut.insert(conn).set_parent(drm_entity);
            let entity = entity_mut.id();
            info!("init monitor {:?} at {entity:?}", name);
        }
        let res_handles = drm.fd.resource_handles().map_err(ResourceHandlesError)?;
        for crtc_handle in res_handles.crtcs() {
            let crtc = drm.fd.get_crtc(*crtc_handle)?;
            debug!("crtc: {:?}", crtc);
        }
        for encoder_handle in res_handles.encoders() {
            let encoder = drm.fd.get_encoder(*encoder_handle)?;
            debug!("encoder: {:?}", encoder);
        }
        for framebuffer_handle in res_handles.framebuffers() {
            let framebuffer = drm.fd.get_framebuffer(*framebuffer_handle)?;
            debug!("framebuffer: {:?}", framebuffer);
        }
    }

    info!("gpu device {gpu_path:?} connected at {:?}", drm_entity);
    udev.device_entity_map.insert(gpu_path, drm_entity);

    commands.entity(drm_entity).insert((drm, gbm));

    Ok(drm_entity)
}

#[tracing::instrument(skip_all)]
pub fn on_udev_event(
    mut udev: NonSendMut<UDevMonitor>,
    mut seat: NonSendMut<SeatState>,
    mut commands: Commands,
    mut drm_query: Query<&mut DrmDevice>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in udev.iter().cloned().collect::<Vec<_>>() {
        match event {
            UDevEvent::Added(device) => {
                let gpu_path = device.devpath().into();
                if let Err(e) =
                    add_device(gpu_path, &mut udev, &mut seat, &mut commands, &mut images)
                {
                    error!("failed to add drm device: {e}");
                }
            }
            UDevEvent::Changed(device) => {
                let gpu_path: PathBuf = device.devpath().into();
                let Some(entity) = udev.device_entity_map.get(&gpu_path) else {
                    continue;
                };
                let Ok(mut drm) = drm_query.get_mut(*entity) else {
                    continue;
                };
                let Ok(events) = drm.connectors_change().map_err(|e| error!("{e}")) else {
                    continue;
                };
                let mut drm_guard = drm.inner.lock().unwrap();
                for change in events {
                    match change {
                        DrmConnectorEvent::Added(info) => {
                            let handle = info.handle();
                            let Ok(conn) = Connector::new(info.clone()).map_err(|e| error!("{e}"))
                            else {
                                continue;
                            };
                            // TODO DrmSurface
                            let name = conn.name.clone();
                            let entity = commands.spawn(conn).id();
                            info!("init monitor {:?} at {entity:?}", name);
                            drm_guard
                                .connectors
                                .get_mut(&handle)
                                .map(|v| v.0 = Some(entity));
                        }
                        DrmConnectorEvent::Removed(info, entity) => {
                            if let Some(entity) = entity {
                                commands.entity(entity).despawn_recursive();
                                drm_guard
                                    .connectors
                                    .get_mut(&info.handle())
                                    .map(|v| v.0 = None);
                            }
                            info!("remove monitor connector at {entity:?}: {:?}", info);
                        }
                    }
                }
            }
            UDevEvent::Removed(device) => {
                if let Some(entity) = udev.device_entity_map.get(&PathBuf::from(device.devpath())) {
                    if drm_query.get(*entity).is_ok() {
                        commands.entity(*entity).despawn_recursive();
                    }
                }
            }
        }
    }
}

pub struct DrmEvent {
    pub entity: Entity,
    pub event: drm::control::Event,
}

#[tracing::instrument(skip_all)]
pub fn recevie_drm_events(
    drm_query: Query<(Entity, &DrmDevice)>,
    mut events: EventWriter<DrmEvent>,
) {
    drm_query.for_each(|(entity, drm)| {
        for event in drm.fd.receive_events().into_iter().flatten() {
            match &event {
                drm::control::Event::Vblank(VblankEvent {
                    frame,
                    time,
                    crtc,
                    user_data,
                }) => {
                    info!("drm event: Vblank({frame:?},{time:?},{crtc:?},{user_data:?})");
                }
                drm::control::Event::PageFlip(PageFlipEvent {
                    frame,
                    duration,
                    crtc,
                }) => {
                    info!("drm event: PageFlip({frame:?},{duration:?},{crtc:?})");
                }
                drm::control::Event::Unknown(data) => {
                    info!("drm event: Unknown({data:?})");
                }
            }
            events.send(DrmEvent { entity, event })
        }
    });
}

pub struct DrmPlugin;
impl Plugin for DrmPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup.on_startup())
            .add_systems(
                (on_udev_event, recevie_drm_events)
                    .chain()
                    .in_set(DWayTTYSet::DrmSystem),
            )
            .add_event::<DrmEvent>();
    }
}
