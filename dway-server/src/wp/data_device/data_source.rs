use crate::prelude::*;

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct WlDataSource {
    #[reflect(ignore)]
    pub raw: wl_data_source::WlDataSource,
}
impl WlDataSource {
    pub fn new(raw: wl_data_source::WlDataSource) -> Self {
        Self { raw }
    }
}
impl Dispatch<wl_data_source::WlDataSource, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &wl_data_source::WlDataSource,
        request: <wl_data_source::WlDataSource as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request{
            wl_data_source::Request::Offer { mime_type } => todo!(),
            wl_data_source::Request::Destroy => todo!(),
            wl_data_source::Request::SetActions { dnd_actions } => todo!(),
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
