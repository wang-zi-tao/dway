use uuid::Uuid;
use wayland_protocols::xdg::activation::v1::server::xdg_activation_token_v1;

use crate::prelude::*;

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct XdgActivationToken {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_activation_token_v1::XdgActivationTokenV1,
    pub seat: Option<Entity>,
    pub app_id: Option<String>,
    pub surface: Option<Entity>,
    pub token: Option<String>,
}
impl XdgActivationToken {
    pub fn new(raw: xdg_activation_token_v1::XdgActivationTokenV1) -> Self {
        Self {
            raw,
            token: None,
            seat: None,
            app_id: None,
            surface: None,
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.token.as_deref()
    }
}
impl Drop for XdgActivationToken {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &xdg_activation_token_v1::XdgActivationTokenV1,
        request: <xdg_activation_token_v1::XdgActivationTokenV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_activation_token_v1::Request::SetSerial { seat, .. } => {
                let mut component = state.get_mut::<XdgActivationToken>(*data).unwrap();
                component.seat = Some(DWay::get_entity(&seat));
            }
            xdg_activation_token_v1::Request::SetAppId { app_id } => {
                let mut component = state.get_mut::<XdgActivationToken>(*data).unwrap();
                component.app_id = Some(app_id);
            }
            xdg_activation_token_v1::Request::SetSurface { surface } => {
                let mut component = state.get_mut::<XdgActivationToken>(*data).unwrap();
                component.surface = Some(DWay::get_entity(&surface));
            }
            xdg_activation_token_v1::Request::Commit => {
                let mut component = state.get_mut::<XdgActivationToken>(*data).unwrap();
                let uuid = Uuid::new_v4().to_string();
                resource.done(uuid.clone());
                component.token = Some(uuid);
            }
            xdg_activation_token_v1::Request::Destroy => {
                state.despawn_object_component::<XdgActivationToken>(*data, resource);
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &xdg_activation_token_v1::XdgActivationTokenV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<XdgActivationToken>(*data, resource);
    }
}
