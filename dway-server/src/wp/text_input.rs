use wayland_protocols::wp::text_input::zv3::server::{
    zwp_text_input_manager_v3, zwp_text_input_v3,
};

use crate::prelude::*;

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwpTextInput {
    #[reflect(ignore)]
    pub raw: zwp_text_input_v3::ZwpTextInputV3,
}
impl ZwpTextInput {
    pub fn new(raw: zwp_text_input_v3::ZwpTextInputV3) -> Self {
        Self { raw }
    }
}
impl Dispatch<zwp_text_input_v3::ZwpTextInputV3, Entity> for DWay {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        resource: &zwp_text_input_v3::ZwpTextInputV3,
        request: <zwp_text_input_v3::ZwpTextInputV3 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_text_input_v3::Request::Destroy => todo!(),
            zwp_text_input_v3::Request::Enable => todo!(),
            zwp_text_input_v3::Request::Disable => todo!(),
            zwp_text_input_v3::Request::SetSurroundingText {
                text: _,
                cursor: _,
                anchor: _,
            } => todo!(),
            zwp_text_input_v3::Request::SetTextChangeCause { cause: _ } => todo!(),
            zwp_text_input_v3::Request::SetContentType { hint: _, purpose: _ } => todo!(),
            zwp_text_input_v3::Request::SetCursorRectangle {
                x: _,
                y: _,
                width: _,
                height: _,
            } => todo!(),
            zwp_text_input_v3::Request::Commit => todo!(),
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

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct ZwpTextInputManager {
    #[reflect(ignore)]
    pub raw: zwp_text_input_manager_v3::ZwpTextInputManagerV3,
}
impl ZwpTextInputManager {
    pub fn new(raw: zwp_text_input_manager_v3::ZwpTextInputManagerV3) -> Self {
        Self { raw }
    }
}
impl Dispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        request: <zwp_text_input_manager_v3::ZwpTextInputManagerV3 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zwp_text_input_manager_v3::Request::Destroy => todo!(),
            zwp_text_input_manager_v3::Request::GetTextInput { id, seat } => {
                state.insert_object(DWay::get_entity(&seat), id, data_init, |o| {
                    ZwpTextInput::new(o)
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
impl GlobalDispatch<zwp_text_input_manager_v3::ZwpTextInputManagerV3, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<zwp_text_input_manager_v3::ZwpTextInputManagerV3>,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, ZwpTextInputManager::new);
    }
}
