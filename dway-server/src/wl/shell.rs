use crate::prelude::*;

impl wayland_server::Dispatch<wl_shell::WlShell, bevy::prelude::Entity, DWay> for DWay {
    fn request(
        _state: &mut DWay,
        _client: &wayland_server::Client,
        _resource: &wl_shell::WlShell,
        request: <wl_shell::WlShell as wayland_server::Resource>::Request,
        _data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_shell::Request::GetShellSurface { id: _, surface: _ } => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_shell::WlShell,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
