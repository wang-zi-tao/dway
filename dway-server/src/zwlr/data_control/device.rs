use wayland_protocols_wlr::data_control::v1::server::zwlr_data_control_device_v1::*;

use crate::{
    clipboard::{ClipboardManager, ClipboardSource},
    prelude::*,
    wp::primary_selection::SourceOfSelection,
    zwlr::data_control::source::ZwlrDataControlSource,
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwlrDataControlDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwlrDataControlDeviceV1,
}
impl ZwlrDataControlDevice {
    pub fn new(raw: ZwlrDataControlDeviceV1) -> Self {
        Self { raw }
    }
}
impl Drop for ZwlrDataControlDevice {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<ZwlrDataControlDeviceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &ZwlrDataControlDeviceV1,
        request: <ZwlrDataControlDeviceV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            Request::SetSelection { source } => {
                if let Some(source) = source {
                    state.connect::<SourceOfSelection>(*data, DWay::get_entity(&source));

                    let mime_types = state
                        .object_component::<ZwlrDataControlSource>(&source)
                        .mime_types
                        .clone();
                    ClipboardManager::add_source(
                        state.world_mut(),
                        ClipboardSource::DataControlSource(source.clone()),
                        mime_types,
                    );
                } else {
                    state.disconnect_all::<SourceOfSelection>(*data);
                }
            }
            Request::Destroy => {
                state.despawn_object(*data, resource);
            }
            Request::SetPrimarySelection { source } => todo!(),
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwlrDataControlDeviceV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
