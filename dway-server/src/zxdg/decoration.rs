use crate::{prelude::*, state::add_global_dispatch, util::unwrap_wl_enum};

#[derive(Component, Reflect)]
pub struct Decoration {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    pub mode: Option<zxdg_toplevel_decoration_v1::Mode>,
}

impl Decoration {
    pub fn new(raw: zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1) -> Self {
        Self { raw, mode: None }
    }
}

impl wayland_server::Dispatch<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1, Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        request: <zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zxdg_toplevel_decoration_v1::Request::Destroy => {
                state.destroy_object(resource);
            }
            zxdg_toplevel_decoration_v1::Request::SetMode { mode } => {
                if let Some(mut this) = state.get_mut::<Decoration>(*data) {
                    this.mode = unwrap_wl_enum(mode);
                };
            }
            zxdg_toplevel_decoration_v1::Request::UnsetMode => {
                if let Some(mut this) = state.get_mut::<Decoration>(*data) {
                    this.mode = None;
                };
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_backend::server::ClientId,
        resource: &zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
        data: &Entity,
    ) {
        state.despawn_object_component::<DecorationManager>(*data, resource);
    }
}

#[derive(Component, Reflect)]
pub struct DecorationManager {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
}

impl DecorationManager {
    pub fn new(raw: zxdg_decoration_manager_v1::ZxdgDecorationManagerV1) -> Self {
        Self { raw }
    }
}

impl wayland_server::Dispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, Entity>
    for DWay
{
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        request: <zxdg_decoration_manager_v1::ZxdgDecorationManagerV1 as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            zxdg_decoration_manager_v1::Request::Destroy => {
                state.destroy_object(resource);
            }
            zxdg_decoration_manager_v1::Request::GetToplevelDecoration { id, toplevel } => {
                state.insert_object(DWay::get_entity(&toplevel), id, data_init, Decoration::new);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_backend::server::ClientId,
        resource: &zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        data: &Entity,
    ) {
        state.despawn_object_component::<DecorationManager>(*data, resource);
    }
}

impl GlobalDispatch<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, Entity> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
        global_data: &Entity,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        state.bind(client, resource, data_init, DecorationManager::new);
    }
}

pub struct DecorationPlugin;
impl Plugin for DecorationPlugin {
    fn build(&self, app: &mut App) {
        add_global_dispatch::<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, 1>(app);
    }
}
