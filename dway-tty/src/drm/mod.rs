pub mod camera;
pub mod connectors;
pub mod planes;
pub mod surface;

use crate::drm::surface::DrmSurface;
use crate::failure::DWayTTYError::*;
use crate::window::create_window;
use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::extract_component::ExtractComponentPlugin;
use bevy::render::Render;
use bevy::render::RenderApp;
use bevy::ui::ui_focus_system;
use bevy::ui::UiSystem;
use bevy::utils::HashMap;
use double_map::DHashMap;
use drm::control::FbCmd2Flags;
use drm::{
    control::{
        atomic::AtomicModeReq, connector, crtc, framebuffer, plane, property, AtomicCommitFlags,
        Device as DrmControlDevice, PropertyValueSet, ResourceHandle, VblankEvent,
    },
    Device,
};
use drm_ffi::drm_format_modifier_blob;
use drm_fourcc::DrmFormat;
use drm_fourcc::DrmFourcc;
use drm_fourcc::DrmModifier;

use gbm::BufferObject;
use nix::libc;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::io;
use std::os::fd::AsFd;
use std::path::Path;
use std::path::PathBuf;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
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

use self::camera::DrmCamera;
use self::connectors::Connector;

pub struct GpuManager {
    pub udev: UDevMonitor,
    pub primary_gpu: Entity,
    pub gpus: HashMap<libc::dev_t, Entity>,
}

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

impl Drop for DrmDevice {
    fn drop(&mut self) {
        let self_guard = self.inner.lock().unwrap();
        if self_guard.privileged {
            let _ = self.fd.release_master_lock();
        }
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
            if let Ok(fourcc) = DrmFourcc::try_from(*format) {
                formats.insert(DrmFormat {
                    code: fourcc,
                    modifier: DrmModifier::Invalid,
                });
            }
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
            DrmDeviceState::Atomic { props, .. } => {
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
            DrmDeviceState::Legacy { .. } => {
                let res_handles = fd.resource_handles().map_err(ResourceHandlesError)?;
                set_connector_state(fd, res_handles.connectors().iter().copied(), false)?;

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
    pub(crate) states: DrmDeviceState,

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

#[derive(Component)]
pub struct ExtractedDrmDevice {
    pub device: DrmDevice,
    pub gbm: GbmDevice,
}

impl ExtractComponent for DrmDevice {
    type Out = ExtractedDrmDevice;
    type QueryData = (&'static DrmDevice, &'static GbmDevice);
    type QueryFilter = ();

    fn extract_component(
        (drm, gbm): bevy::ecs::query::QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(ExtractedDrmDevice {
            device: drm.clone(),
            gbm: gbm.clone(),
        })
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
                states,
                connector_crtc_map: Default::default(),
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
            .map(Connector::new)
            .try_collect()?;
        Ok(connectors)
    }

    pub fn connectors_change(&self) -> Result<SmallVec<[DrmConnectorEvent; 1]>> {
        self.inner.lock().unwrap().connectors_change(&self.fd)
    }

    pub fn create_framebuffer(&self, buffer: &BufferObject<()>) -> Result<framebuffer::Handle> {
        let plane_count = buffer.plane_count()?;
        let handle = self.add_planar_framebuffer(buffer, FbCmd2Flags::MODIFIERS);
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
            device
                .property_value("ID_SEAT")
                .map(|x| x.to_string_lossy())
                .unwrap_or_else(|| Cow::from("seat0"))
                == seat.name
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
    debug!(
        r"DRM Debugging method:
    echo 0x19F | sudo tee /sys/module/drm/parameters/debug
    sudo dmesg -C
    dmesg -w
    "
    );

    for gpu_path in all_gpus(&seat).unwrap() {
        if let Err(e) = add_device(gpu_path, &mut udev, &mut seat, &mut commands, &mut images) {
            error!("failed to add drm device: {e}");
        }
    }
}

pub fn add_connector(
    conn: Connector,
    drm: &DrmDevice,
    drm_entity: Entity,
    images: &mut Assets<Image>,
    commands: &mut Commands,
) {
    trace!("conn: {:?}", conn);
    let mut entity_mut = commands.spawn_empty();

    {
        let mut guard = drm.inner.lock().unwrap();
        guard
            .connectors
            .get_mut(&conn.info.handle())
            .map(|v| v.0 = Some(entity_mut.id()));
    }

    let Ok(surface) =
        DrmSurface::new(drm, &conn, images).map_err(|e| error!("failed to connect screen: {e}"))
    else {
        return;
    };
    trace!("drm surface: {:?}", &surface);

    let name = conn.name.clone();
    let window = create_window(&conn, &surface);
    entity_mut
        .insert((window, surface, conn, Name::new(name.clone())))
        .set_parent(drm_entity);
    let entity = entity_mut.id();
    info!("init monitor {:?} at {entity:?}", name);
}

pub fn add_device(
    gpu_path: PathBuf,
    udev: &mut UDevMonitor,
    seat: &mut SeatState,
    commands: &mut Commands,
    images: &mut Assets<Image>,
) -> Result<Entity> {
    let _span = span!(Level::ERROR,"init drm device",path=%gpu_path.to_string_lossy()).entered();

    debug!("open drm device");
    let drm_fd = seat.open_device(&gpu_path)?;
    let drm = DrmDevice::new(drm_fd, gpu_path.clone())?;
    let gbm = GbmDevice::new(drm.fd.clone())?;

    let connectors = drm.connectors()?;

    let drm_entity = commands.spawn_empty().id();

    {
        for conn in connectors {
            add_connector(conn, &drm, drm_entity, images, commands);
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
                let Some(drm_entity) = udev.device_entity_map.get(&gpu_path) else {
                    continue;
                };
                let Ok(drm) = drm_query.get_mut(*drm_entity) else {
                    continue;
                };
                let Ok(events) = drm.connectors_change().map_err(|e| error!("{e}")) else {
                    continue;
                };
                let mut drm_guard = drm.inner.lock().unwrap();
                for change in events {
                    match change {
                        DrmConnectorEvent::Added(info) => {
                            let Ok(info) = Connector::new(info)
                                .map_err(|e| error!("failed to connect screen: {e}"))
                            else {
                                continue;
                            };
                            add_connector(info, &drm, *drm_entity, &mut images, &mut commands);
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

#[derive(Event)]
pub struct DrmEvent {
    pub entity: Entity,
    pub event: drm::control::Event,
}

#[tracing::instrument(skip_all)]
pub fn recevie_drm_events(
    drm_query: Query<(Entity, &DrmDevice, Option<&Children>)>,
    surface_query: Query<&DrmSurface>,
    mut events_writer: EventWriter<DrmEvent>,
) {
    drm_query.iter().for_each(|(entity, drm, children)| {
        let events = match drm.fd.receive_events() {
            Ok(o) => o,
            Err(e) => {
                error!(?entity, "failed to receive drm events: {e}");
                return;
            }
        };
        for event in events {
            match &event {
                drm::control::Event::Vblank(VblankEvent {
                    frame,
                    time,
                    crtc,
                    user_data,
                }) => {
                    debug!("drm event: Vblank({frame:?},{time:?},{crtc:?},{user_data:?})");
                }
                drm::control::Event::PageFlip(e) => {
                    debug!(
                        "drm event: PageFlip({:?},{:?},{:?})",
                        &e.frame, &e.duration, &e.crtc
                    );
                    if let Some(children) = children {
                        for entity in children.iter() {
                            if let Ok(surface) = surface_query.get(*entity) {
                                let mut surface_guard = surface.inner.lock().unwrap();
                                if surface_guard.crtc == e.crtc {
                                    surface_guard.on_page_flip(e);
                                }
                            }
                        }
                    }
                }
                drm::control::Event::Unknown(data) => {
                    debug!("drm event: Unknown({data:?})");
                }
            }
            events_writer.send(DrmEvent { entity, event });
        }
    });
}

pub struct DrmPlugin;
impl Plugin for DrmPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, setup)
            .add_plugins(ExtractComponentPlugin::<DrmDevice>::default())
            .add_systems(First, on_udev_event.in_set(DWayTTYSet::UdevSystem))
            .add_systems(
                PreUpdate,
                (
                    camera::before_ui_focus.before(ui_focus_system),
                    camera::after_ui_focus.after(ui_focus_system),
                )
                    .in_set(UiSystem::Focus),
            )
            .register_type::<DrmCamera>();
        app.sub_app_mut(RenderApp)
            .add_systems(
                Render,
                recevie_drm_events.in_set(DWayTTYSet::DrmEventSystem),
            )
            .add_event::<DrmEvent>();
    }
}

#[cfg(test)]
mod test {
    use bevy::prelude::*;
    use dway_util::eventloop::{EventLoopPlugin, EventLoopPluginMode};

    use crate::{
        schedule::DWayTtySchedulePlugin, seat::SeatPlugin, test::test_suite_plugins,
        udev::UDevPlugin,
    };

    use super::DrmPlugin;

    #[test]
    pub fn test_drm_plugin() {
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
        app.run();
    }
}
