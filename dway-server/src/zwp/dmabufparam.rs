use std::sync::{Arc, Mutex};

use drm_fourcc::DrmModifier;
use wayland_backend::io_lifetimes::OwnedFd;
use wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_buffer_params_v1::Flags;

use crate::prelude::*;

#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmaBuffer {
    pub raw: wl_buffer::WlBuffer,
    pub size: IVec2,
    pub format: u32,
    #[reflect(ignore)]
    pub flags: WEnum<Flags>,
    pub planes: Arc<Mutex<DmaBufferPlanes>>,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmaBufferPlane {
    pub fd: OwnedFd,
    pub plane_idx: u32,
    pub offset: u32,
    pub stride: u32,
    pub modifier_hi: u32,
    pub modifier_lo: u32,
}
impl DmaBufferPlane {
    pub fn modifier(&self) -> DrmModifier {
        DrmModifier::from(((self.modifier_hi as u64) << 32) | self.modifier_lo as u64)
    }
}

#[derive(Reflect, Debug, Default)]
#[reflect(Debug)]
pub struct DmaBufferPlanes {
    pub list: Vec<DmaBufferPlane>,
}

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmaBufferParams {
    #[reflect(ignore)]
    pub raw: zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    pub planes: Arc<Mutex<DmaBufferPlanes>>,
}
impl DmaBufferParams {
    pub fn new(raw: zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1) -> Self {
        Self {
            raw,
            planes: Default::default(),
        }
    }
}
impl Dispatch<zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        request: <zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_linux_buffer_params_v1::Request::Destroy => {
                state.despawn_object(*data, resource.id());
            }
            zwp_linux_buffer_params_v1::Request::Add {
                fd,
                plane_idx,
                offset,
                stride,
                modifier_hi,
                modifier_lo,
            } => {
                let params = state.get_mut::<DmaBufferParams>(*data).unwrap();
                let mut planes = params.planes.lock().unwrap();
                planes.list.push(DmaBufferPlane {
                    fd,
                    plane_idx,
                    offset,
                    stride,
                    modifier_hi,
                    modifier_lo,
                })
            }
            zwp_linux_buffer_params_v1::Request::Create {
                width,
                height,
                format,
                flags,
            } => {
                let mut entity = state.spawn_empty();
                let buffer = match client.create_resource::<wl_buffer::WlBuffer, Entity, DWay>(
                    dhandle,
                    1,
                    entity.id(),
                ) {
                    Ok(o) => o,
                    Err(e) => {
                        error!("failed to create wl_buffer: {e}");
                        resource.failed();
                        return;
                    }
                };
                resource.created(&buffer);
                entity.insert(DmaBuffer {
                    raw: buffer,
                    size: IVec2::new(width, height),
                    format,
                    flags,
                    planes: Default::default(),
                });
            }
            zwp_linux_buffer_params_v1::Request::CreateImmed {
                buffer_id,
                width,
                height,
                format,
                flags,
            } => {
                state.spawn_child_object_bundle(*data, buffer_id, data_init, |o| DmaBuffer {
                    raw: o,
                    size: IVec2::new(width, height),
                    format,
                    flags,
                    planes: Default::default(),
                });
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
