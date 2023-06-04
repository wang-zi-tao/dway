use std::sync::Arc;

use crate::{prelude::*, resource::ResourceWrapper};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPopup {
    #[reflect(ignore)]
    raw: xdg_popup::XdgPopup,
}

impl XdgPopup {
    pub fn new(raw: xdg_popup::XdgPopup) -> Self {
        Self { raw }
    }
}
impl ResourceWrapper for XdgPopup {
    type Resource = xdg_popup::XdgPopup;

    fn get_resource(&self) -> &Self::Resource {
        &self.raw
    }
}

#[derive(Resource)]
pub struct PopupDelegate(pub GlobalId);
delegate_dispatch!(DWay: [xdg_popup::XdgPopup: Entity] => PopupDelegate);
impl wayland_server::Dispatch<xdg_popup::XdgPopup, bevy::prelude::Entity, DWay> for PopupDelegate {
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &xdg_popup::XdgPopup,
        request: <xdg_popup::XdgPopup as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            xdg_popup::Request::Destroy => todo!(),
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: wayland_backend::server::ObjectId,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data,resource);
    }
}

impl wayland_server::GlobalDispatch<xdg_popup::XdgPopup, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<xdg_popup::XdgPopup>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}

pub struct XdgPopupPlugin(pub Arc<DisplayHandle>);
impl Plugin for XdgPopupPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PopupDelegate(
            self.0.create_global::<DWay, xdg_popup::XdgPopup, ()>(1, ()),
        ));
        app.add_event::<Insert<XdgPopup>>();
        app.register_type::<XdgPopup>();
    }
}
