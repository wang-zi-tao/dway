use wayland_protocols::wp::primary_selection::zv1::server::zwp_primary_selection_device_v1::{
    self, ZwpPrimarySelectionDeviceV1,
};

use crate::{
    clipboard::{ClipboardManager, ClipboardSource},
    prelude::*,
    wp::primary_selection::{source::PrimarySelectionSource, SourceOfSelection},
};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct PrimarySelectionDevice {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: ZwpPrimarySelectionDeviceV1,
    pub serial: Option<u32>,
}
impl PrimarySelectionDevice {
    pub fn new(raw: ZwpPrimarySelectionDeviceV1) -> Self {
        Self { raw, serial: None }
    }
}
impl Dispatch<ZwpPrimarySelectionDeviceV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &ZwpPrimarySelectionDeviceV1,
        request: <ZwpPrimarySelectionDeviceV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_primary_selection_device_v1::Request::SetSelection { source, serial } => {
                if let Some(source) = source {
                    state.connect::<SourceOfSelection>(*data, DWay::get_entity(&source));
                    state.with_component(resource, |c: &mut PrimarySelectionDevice| {
                        c.serial = Some(serial);
                    });

                    let mime_types = state
                        .object_component::<PrimarySelectionSource>(&source)
                        .mime_types
                        .clone();
                    ClipboardManager::add_source(
                        state.world_mut(),
                        ClipboardSource::PrimarySelectionSource(source.clone()),
                        mime_types,
                    );
                } else {
                    state.disconnect_all::<SourceOfSelection>(*data);
                    state.with_component(resource, |c: &mut PrimarySelectionDevice| {
                        c.serial = None;
                    });
                }
            }
            zwp_primary_selection_device_v1::Request::Destroy => {
                state.despawn_object(*data, resource);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &ZwpPrimarySelectionDeviceV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
