use drm_fourcc::DrmFourcc;

use crate::{
    prelude::*,
    zwp::{
        dambuffeedback::{DmabufFeedback, PeddingDmabufFeedback},
        dmabufparam::DmaBufferParams,
    },
};

relationship!(DmaBufferHasFeedback=>FeedbackList-<DmaBufferRef);
relationship!(DmaBufferAttachSurface=>SurfaceListForDmaBuffer-<DmaBufferRefForSurface);

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwpDmaBufferFactory {
    #[reflect(ignore)]
    pub raw: zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
}
impl ZwpDmaBufferFactory {
    pub fn new(raw: zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1) -> Self {
        Self { raw }
    }
}
impl Dispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
        request: <zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_linux_dmabuf_v1::Request::Destroy => {
                state.despawn_object_component::<ZwpDmaBufferFactory>(*data, resource.id());
            }
            zwp_linux_dmabuf_v1::Request::CreateParams { params_id } => {
                state.insert_object(*data, params_id, data_init, DmaBufferParams::new);
            }
            zwp_linux_dmabuf_v1::Request::GetDefaultFeedback { id } => {
                let entity = state
                    .spawn((id, data_init, |o| {
                        (DmabufFeedback::new(o), PeddingDmabufFeedback)
                    }))
                    .set_parent(*data)
                    .id();
                state.connect::<DmaBufferHasFeedback>(entity, *data);
            }
            zwp_linux_dmabuf_v1::Request::GetSurfaceFeedback { id, surface } => {
                let entity = state
                    .insert(
                        DWay::get_entity(&surface),
                        (id, data_init, |o| {
                            (DmabufFeedback::new(o), PeddingDmabufFeedback)
                        }),
                    )
                    .id();
                state.connect::<DmaBufferAttachSurface>(entity, *data);
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
        state.despawn_object_component::<ZwpDmaBufferFactory>(*data, resource);
    }
}
impl GlobalDispatch<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1, Entity> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1>,
        global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |o| {
            o.format(DrmFourcc::Argb8888 as u32);
            o.format(DrmFourcc::Xrgb8888 as u32);
            o.modifier(DrmFourcc::Argb8888 as u32, 0x00ffffff, 0xffffffff);
            o.modifier(DrmFourcc::Xrgb8888 as u32, 0x00ffffff, 0xffffffff);
            if o.version() < zwp_linux_dmabuf_v1::REQ_GET_DEFAULT_FEEDBACK_SINCE {
                dbg!(o.version());
                todo!();
            }
            ZwpDmaBufferFactory::new(o)
        });
    }
}

pub struct DWayDmaBufferFactoryPlugin;
impl Plugin for DWayDmaBufferFactoryPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<DmaBufferHasFeedback>();
        app.register_relation::<DmaBufferAttachSurface>();
        app.register_type::<ZwpDmaBufferFactory>();
        app.add_system(create_global_system_config::<
            zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1,
            4,
        >());
    }
}
