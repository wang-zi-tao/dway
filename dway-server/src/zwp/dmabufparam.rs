use std::{mem::take, os::fd::OwnedFd};

use drm_fourcc::DrmModifier;
use wayland_protocols::wp::linux_dmabuf::zv1::server::zwp_linux_buffer_params_v1::Flags;

use crate::{
    prelude::*,
    render::{DWayRenderRequest, DWayServerRenderClient, ImportDmaBufferRequest},
};

#[derive(Component, Clone, Reflect, Debug)]
#[reflect(Debug)]
pub struct DmaBuffer {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: Option<wl_buffer::WlBuffer>,
    pub size: IVec2,
    pub format: u32,
    #[reflect(ignore, default = "unimplemented")]
    pub flags: WEnum<Flags>,
}

#[derive(Component, Debug)]
pub struct DmaBufferPlane {
    pub fd: OwnedFd,
    pub plane_idx: u32,
    pub offset: u32,
    pub stride: u32,
    pub modifier: DrmModifier,
}

#[derive(Component, Debug)]
pub struct DmaBufferParams {
    pub raw: zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
    pub planes: Vec<DmaBufferPlane>,
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
                state.despawn_object(*data, resource);
            }
            zwp_linux_buffer_params_v1::Request::Add {
                fd,
                plane_idx,
                offset,
                stride,
                modifier_hi,
                modifier_lo,
            } => {
                let mut params = state.get_mut::<DmaBufferParams>(*data).unwrap();
                let modifier = DrmModifier::from(((modifier_hi as u64) << 32) | modifier_lo as u64);
                params.planes.push(DmaBufferPlane {
                    fd,
                    plane_idx,
                    offset,
                    stride,
                    modifier,
                })
            }
            zwp_linux_buffer_params_v1::Request::Create {
                width,
                height,
                format,
                flags,
            } => {
                let buffer_entity = state.spawn(ChildOf(DWay::client_entity(client))).id();
                let mut planes = take(&mut state.get_mut::<DmaBufferParams>(*data).unwrap().planes);
                planes.sort_by_key(|p| p.plane_idx);
                let render_client = state.resource::<DWayServerRenderClient>();

                render_client
                    .request_tx
                    .push(DWayRenderRequest::ImportDmaBuffer(ImportDmaBufferRequest {
                        buffer: None,
                        client: client.clone(),
                        display: dhandle.clone(),
                        size: IVec2::new(width, height).as_uvec2(),
                        format,
                        flags,
                        planes,
                        buffer_entity,
                        params: resource.clone(),
                    }));

                state.entity_mut(buffer_entity).insert(DmaBuffer {
                    raw: None,
                    size: IVec2::new(width, height),
                    format,
                    flags,
                });
            }
            zwp_linux_buffer_params_v1::Request::CreateImmed {
                buffer_id,
                width,
                height,
                format,
                flags,
            } => {
                let buffer_entity = state.spawn(ChildOf(DWay::client_entity(client))).id();
                let mut planes = take(&mut state.get_mut::<DmaBufferParams>(*data).unwrap().planes);
                planes.sort_by_key(|p| p.plane_idx);
                let render_client = state.resource::<DWayServerRenderClient>();

                let buffer = data_init.init(buffer_id, buffer_entity);

                render_client
                    .request_tx
                    .push(DWayRenderRequest::ImportDmaBuffer(ImportDmaBufferRequest {
                        buffer: Some(buffer.clone()),
                        client: client.clone(),
                        display: dhandle.clone(),
                        size: IVec2::new(width, height).as_uvec2(),
                        format,
                        flags,
                        planes,
                        buffer_entity,
                        params: resource.clone(),
                    }));

                state.entity_mut(buffer_entity).insert(DmaBuffer {
                    raw: None,
                    size: IVec2::new(width, height),
                    format,
                    flags,
                });
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &zwp_linux_buffer_params_v1::ZwpLinuxBufferParamsV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
