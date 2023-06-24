use wayland_protocols::xdg::activation::v1::server::xdg_activation_token_v1;

use crate::prelude::*;

impl wayland_server::Dispatch<xdg_activation_token_v1::XdgActivationTokenV1, bevy::prelude::Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &xdg_activation_token_v1::XdgActivationTokenV1,
        request: <xdg_activation_token_v1::XdgActivationTokenV1 as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            xdg_activation_token_v1::Request::SetSerial { serial, seat } => todo!(),
            xdg_activation_token_v1::Request::SetAppId { app_id } => todo!(),
            xdg_activation_token_v1::Request::SetSurface { surface } => todo!(),
            xdg_activation_token_v1::Request::Commit => todo!(),
            xdg_activation_token_v1::Request::Destroy => todo!(),
            _ => todo!(),
        }
    }
}

#[derive(Component)]
pub struct XdgActivationToken {
    pub raw: xdg_activation_token_v1::XdgActivationTokenV1,
}

impl
    wayland_server::GlobalDispatch<
        xdg_activation_token_v1::XdgActivationTokenV1,
        bevy::prelude::Entity,
    > for DWay
{
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<xdg_activation_token_v1::XdgActivationTokenV1>,
        global_data: &bevy::prelude::Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, |raw| XdgActivationToken {
            raw,
        });
    }
}
