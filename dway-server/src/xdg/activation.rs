use crate::{prelude::*, xdg::activation_token::XdgActivationToken};

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct SurfaceActivate {
    pub token: String,
}

impl SurfaceActivate {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[derive(Component, Reflect, Debug)]
#[reflect(Debug)]
pub struct XdgActivation {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_activation_v1::XdgActivationV1,
}
impl XdgActivation {
    pub fn new(raw: xdg_activation_v1::XdgActivationV1) -> Self {
        Self { raw }
    }
}
impl Drop for XdgActivation {
    fn drop(&mut self) {
        trace!(entity = ?DWay::get_entity(&self.raw),resource = ?self.raw.id(),"drop wayland resource");
    }
}
impl Dispatch<xdg_activation_v1::XdgActivationV1, Entity> for DWay {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        resource: &xdg_activation_v1::XdgActivationV1,
        request: <xdg_activation_v1::XdgActivationV1 as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_activation_v1::Request::Destroy => {
                state.despawn_object_component::<XdgActivation>(*data, resource);
            }
            xdg_activation_v1::Request::GetActivationToken { id } => {
                state.insert_object(*data, id, data_init, XdgActivationToken::new);
            }
            xdg_activation_v1::Request::Activate { token, surface } => {
                state
                    .entity_mut(DWay::get_entity(&surface))
                    .insert(SurfaceActivate::new(token));
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &xdg_activation_v1::XdgActivationV1,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
impl GlobalDispatch<xdg_activation_v1::XdgActivationV1, Entity> for DWay {
    fn bind(
        state: &mut Self,
        _handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<xdg_activation_v1::XdgActivationV1>,
        _global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, XdgActivation::new);
    }
}
