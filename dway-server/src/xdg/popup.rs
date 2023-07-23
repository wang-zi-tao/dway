use std::sync::Arc;

use bevy_relationship::Connectable;
use wayland_protocols::xdg::shell::server::xdg_positioner::{Anchor, Gravity};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::{grab::PointerGrab, seat::PointerList},
    prelude::*,
    resource::ResourceWrapper,
    state::create_global_system_config,
    util::rect::IRect,
};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPopup {
    #[reflect(ignore)]
    pub raw: xdg_popup::XdgPopup,
    pub anchor_rect: Option<IRect>,
    pub constraint_adjustment: Option<u32>,
    #[reflect(ignore)]
    pub anchor_kind: Option<Anchor>,
    #[reflect(ignore)]
    pub gravity: Option<Gravity>,
    pub is_relative: bool,
    pub send_configure: bool,
}

impl XdgPopup {
    pub fn new(
        raw: xdg_popup::XdgPopup,
        anchor_rect: Option<IRect>,
        constraint_adjustment: Option<u32>,
        anchor_kind: Option<Anchor>,
        gravity: Option<Gravity>,
        is_relative: bool,
    ) -> Self {
        Self {
            raw,
            anchor_rect,
            constraint_adjustment,
            anchor_kind,
            gravity,
            is_relative,
            send_configure: false,
        }
    }
}
impl ResourceWrapper for XdgPopup {
    type Resource = xdg_popup::XdgPopup;

    fn get_resource(&self) -> &Self::Resource {
        &self.raw
    }
}
#[derive(Bundle)]
pub struct XdgPopupBundle {
    pub raw: XdgPopup,
    pub geometry: Geometry,
    pub global_geometry: GlobalGeometry,
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
            xdg_popup::Request::Destroy => {
                state.despawn(*data);
            }
            xdg_popup::Request::Grab { seat, serial } => {
                let pointer_list = state
                    .world_mut()
                    .get::<PointerList>(DWay::get_entity(&seat))
                    .unwrap()
                    .clone();
                for pointer in pointer_list.iter() {
                    if let Some(mut grab) = state.world_mut().get_mut::<PointerGrab>(pointer) {
                        *grab = PointerGrab::OnPopup {
                            surface: *data,
                            serial,
                        }
                    }
                }
            }
            xdg_popup::Request::Reposition { positioner, token } => todo!(),
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

pub struct XdgPopupPlugin;
impl Plugin for XdgPopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Insert<XdgPopup>>();
        app.register_type::<XdgPopup>();
    }
}
