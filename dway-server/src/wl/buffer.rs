use crate::{
    prelude::*,
    state::{add_global_dispatch, EntityFactory},
};
use drm_fourcc::DrmModifier;
use khronos_egl::EGLDisplay;
use nix::sys::mman;
use std::{
    num::NonZeroUsize,
    os::fd::OwnedFd,
    ptr::NonNull,
    sync::{Arc, RwLock},
};
use wayland_server::Resource;
use super::surface::AttachedBy;

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlShmBuffer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_buffer::WlBuffer,
    pub offset: i32,
    pub size: IVec2,
    pub stride: i32,
    #[reflect(ignore, default = "unimplemented")]
    pub format: wl_shm::Format,
    #[reflect(ignore, default = "unimplemented")]
    pub pool: Arc<RwLock<WlShmPoolInner>>,
}
#[derive(Bundle)]
pub struct WlMemoryBufferBundle {
    resource: WlShmBuffer,
    attach_by: AttachedBy,
}

impl WlMemoryBufferBundle {
    pub fn new(resource: WlShmBuffer) -> Self {
        Self {
            resource,
            attach_by: Default::default(),
        }
    }
}

#[derive(Component, Clone)]
pub struct UninitedWlBuffer {
    pub raw: wl_buffer::WlBuffer,
}

impl UninitedWlBuffer {
    pub fn new(raw: wl_buffer::WlBuffer) -> Self {
        Self { raw }
    }
}

#[derive(Debug)]
pub struct Plane {
    pub fd: OwnedFd,
    /// The plane index
    pub plane_idx: u32,
    /// Offset from the start of the Fd
    pub offset: u32,
    /// Stride for this plane
    pub stride: u32,
    /// Modifier for this plane
    pub modifier: DrmModifier,
}
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DmabufFlags: u32 {
        const Y_INVERT = 1;
        const INTERLACED = 2;
        const BOTTOM_FIRST = 4;
    }
}
pub struct EGLBufferInner {
    pub handle: EGLDisplay,
    pub _native: Box<dyn std::any::Any + 'static>,
}
unsafe impl Send for EGLBufferInner {}
unsafe impl Sync for EGLBufferInner {}
#[derive(Component, Clone)]
pub struct EGLBuffer {
    pub display: Option<Entity>,
    pub inner: Arc<EGLBufferInner>,
}
const FORMATS: [wl_shm::Format; 14] = [
    wl_shm::Format::Argb8888,
    wl_shm::Format::Xrgb8888,
    wl_shm::Format::Rgb565,
    wl_shm::Format::Yuv420,
    wl_shm::Format::Yuv444,
    wl_shm::Format::Nv12,
    wl_shm::Format::Yuyv,
    wl_shm::Format::Xyuv8888,
    wl_shm::Format::Abgr2101010,
    wl_shm::Format::Xbgr2101010,
    wl_shm::Format::Abgr16161616f,
    wl_shm::Format::Xbgr16161616f,
    wl_shm::Format::Abgr16161616,
    wl_shm::Format::Xbgr16161616,
];

#[derive(Debug)]
pub struct WlShmPoolInner {
    pub ptr: NonNull<u8>,
    pub size: usize,
    pub fd: OwnedFd,
}
unsafe impl Sync for WlShmPoolInner {}
unsafe impl Send for WlShmPoolInner {}
impl Drop for WlShmPoolInner {
    fn drop(&mut self) {
        if let Err(e) = unsafe { mman::munmap(self.ptr.as_ptr().cast(), self.size) } {
            error!(error=%e,"unmap failed");
        }
        debug!("unmap wl_shm_pool");
    }
}
#[derive(Component)]
pub struct WlShm {
    pub raw: wl_shm::WlShm,
}
#[derive(Component, Clone, Debug)]
pub struct WlShmPool {
    pub raw: wl_shm_pool::WlShmPool,
    pub inner: Arc<RwLock<WlShmPoolInner>>,
}

pub struct BufferDelegate;
delegate_dispatch!(DWay: [wl_buffer::WlBuffer: Entity] => BufferDelegate);
impl wayland_server::Dispatch<wl_buffer::WlBuffer, bevy::prelude::Entity, DWay> for BufferDelegate {
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wl_buffer::WlBuffer,
        request: <wl_buffer::WlBuffer as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_buffer::Request::Destroy => {
                trace!(entity=?data,resource=%WlResource::id(resource),"destroy buffer");
                state.despawn_tree(*data);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_buffer::WlBuffer,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
delegate_dispatch!(DWay: [wl_shm::WlShm: Entity] => BufferDelegate);
impl wayland_server::Dispatch<wl_shm::WlShm, Entity, DWay> for BufferDelegate {
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wl_shm::WlShm,
        request: <wl_shm::WlShm as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        use wl_shm::Error;
        match request {
            wl_shm::Request::CreatePool { id, fd, size } => {
                if size <= 0 {
                    resource.post_error(Error::InvalidStride, "invalid wl_shm_pool size");
                    return;
                }
                let ret = unsafe {
                    mman::mmap(
                        None,
                        NonZeroUsize::new(size as usize).unwrap(),
                        mman::ProtFlags::PROT_READ | mman::ProtFlags::PROT_WRITE,
                        mman::MapFlags::MAP_SHARED,
                        Some(&fd),
                        0,
                    )
                };
                let ptr = match ret {
                    Ok(o) => o,
                    Err(_e) => {
                        error!("failed to call mmap on {fd:?}");
                        return;
                    }
                };
                let poolinner = Arc::new(RwLock::new(WlShmPoolInner {
                    ptr: NonNull::new(ptr.cast()).unwrap(),
                    size: size as usize,
                    fd,
                }));
                state.spawn_child_object(*data, id, data_init, |o| WlShmPool {
                    raw: o,
                    inner: poolinner,
                });
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_shm::WlShm,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<WlShm>(*data, resource);
    }
}
delegate_dispatch!(DWay: [wl_shm_pool::WlShmPool: Entity] => BufferDelegate);
impl wayland_server::Dispatch<wl_shm_pool::WlShmPool, bevy::prelude::Entity, DWay>
    for BufferDelegate
{
    fn request(
        state: &mut DWay,
        _client: &wayland_server::Client,
        resource: &wl_shm_pool::WlShmPool,
        request: <wl_shm_pool::WlShmPool as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset,
                width,
                height,
                stride,
                format,
            } => {
                let size = state
                    .with_component(resource, |c: &mut WlShmPool| c.inner.read().unwrap().size)
                    .unwrap_or_default();
                let message = if offset < 0 {
                    Some("offset must not be negative".to_string())
                } else if width <= 0 || height <= 0 {
                    Some(format!("invalid width or height ({}x{})", width, height))
                } else if stride < width {
                    Some(format!(
                        "width must not be larger than stride (width {}, stride {})",
                        width, stride
                    ))
                } else if (i32::MAX / stride) < height {
                    Some(format!(
                        "height is too large for stride (max {})",
                        i32::MAX / stride
                    ))
                } else if offset > size as i32 - (stride * height) {
                    Some("offset is too large".to_string())
                } else {
                    None
                };

                if let Some(message) = message {
                    resource.post_error(wl_shm::Error::InvalidStride, message);
                    return;
                }
                match format {
                    WEnum::Unknown(unknown) => {
                        resource.post_error(
                            wl_shm::Error::InvalidFormat,
                            format!("unknown format 0x{:x}", unknown),
                        );
                        return;
                    }
                    WEnum::Value(format) => {
                        if !FORMATS.contains(&format) {
                            resource.post_error(
                                wl_shm::Error::InvalidFormat,
                                format!("format {:?} not supported", format),
                            );
                        }
                    }
                }
                let format = match format {
                    WEnum::Value(format) => format,
                    WEnum::Unknown(format) => {
                        error!("unknown format {format}");
                        return;
                    }
                };
                let Some(pool) =
                    state.with_component(resource, |pool: &mut WlShmPool| pool.inner.clone())
                else {
                    return;
                };
                let Some(parent) = state.with_component(resource, |p: &mut Parent| p.get()) else {
                    return;
                };
                state.spawn(
                    (id, data_init, |o| {
                        WlMemoryBufferBundle::new(WlShmBuffer {
                            raw: o,
                            offset,
                            size: IVec2::new(width, height),
                            stride,
                            format,
                            pool,
                        })
                    })
                        .with_parent(parent),
                );
            }
            wl_shm_pool::Request::Destroy => {
                trace!(resource=%WlResource::id(resource),"destroy wl_shm_pool");
                state.despawn_tree(*data);
            }
            wl_shm_pool::Request::Resize { size } => {
                state.with_component(resource, |c: &mut WlShmPool| {
                    if size <= 0 {
                        resource.post_error(wl_shm::Error::InvalidFd, "invalid wl_shm_pool size");
                        return;
                    }
                    if c.inner.read().unwrap().size >= size as usize {
                        resource.post_error(wl_shm::Error::InvalidFd, "cannot shrink wl_shm_pool");
                        return;
                    }
                    let mut inner = c.inner.write().unwrap();
                    match unsafe {
                        mman::mmap(
                            None,
                            NonZeroUsize::new(size as usize).unwrap(),
                            mman::ProtFlags::PROT_READ | mman::ProtFlags::PROT_WRITE,
                            mman::MapFlags::MAP_SHARED,
                            Some(&inner.fd),
                            0,
                        )
                    } {
                        Ok(ptr) => {
                            inner.ptr = NonNull::new(ptr.cast()).unwrap();
                            inner.size = size as usize;
                        }
                        Err(e) => {
                            resource.post_error(wl_shm::Error::InvalidFd, "mremap failed");
                            error!(error=%e,"unmap failed");
                        }
                    }
                });
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_shm_pool::WlShmPool,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl wayland_server::GlobalDispatch<wl_shm::WlShm, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wl_shm::WlShm>,
        _global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            for format in FORMATS {
                o.format(format);
            }
            WlShm { raw: o }
        });
    }
}

pub struct WlBufferPlugin;
impl Plugin for WlBufferPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<wl_shm::WlShm, 1>(app);
        app.register_type::<WlShmBuffer>();
    }
}
