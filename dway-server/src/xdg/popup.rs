use std::sync::Arc;

use bevy_relationship::Connectable;
use wayland_protocols::xdg::shell::server::xdg_positioner::{Anchor, Gravity};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::{
        grab::PointerGrab,
        pointer::WlPointer,
        seat::{KeyboardList, PointerList},
    },
    prelude::*,
    resource::ResourceWrapper,
    state::create_global_system_config,
    util::rect::IRect,
    wl::surface::WlSurface,
    xdg::{positioner::XdgPositioner, DWayWindow},
};

use super::positioner::{self, Positioner};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPopup {
    #[reflect(ignore)]
    pub raw: xdg_popup::XdgPopup,
    pub send_configure: bool,
    pub positioner: Positioner,
}

impl XdgPopup {
    pub fn new(raw: xdg_popup::XdgPopup, positioner: Positioner) -> Self {
        Self {
            raw,
            send_configure: false,
            positioner,
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
    pub window: DWayWindow,
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
        let span = span!(Level::ERROR, "request", entity=?data, resource=%WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_popup::Request::Destroy => {
                state.despawn(*data);
                state.send_event(Destroy::<DWayWindow>::new(*data));
            }
            xdg_popup::Request::Grab { seat, serial } => {
                let seat_entity = DWay::get_entity(&seat);
                let Some(pointer_list) = state.world_mut().get::<PointerList>(seat_entity).cloned()
                else {
                    return;
                };
                for pointer_entity in pointer_list.iter() {
                    state.query::<(&mut PointerGrab, &Geometry, &mut WlPointer), _, _>(
                        pointer_entity,
                        |(mut grab, pointer_rect, mut pointer)| {
                            *grab = PointerGrab::OnPopup {
                                surface: *data,
                                pressed: false,
                                serial,
                            };
                            pointer.unset_grab();
                        },
                    );
                }
                // let keyboard_list = state.get::<KeyboardList>(seat_entity).unwrap().clone();
            }
            xdg_popup::Request::Reposition { positioner, token } => {
                let positioner =
                    state.with_component(&positioner, |c: &mut XdgPositioner| c.positioner.clone());
                state.query::<(&mut XdgPopup, &mut Geometry), _, _>(*data, |(mut p, mut g)| {
                    p.positioner = positioner;
                    g.geometry = IRect::from_pos_size(
                        p.positioner.anchor_rect.unwrap_or_default().max,
                        IVec2::default(),
                    );
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

pub struct XdgPopupPlugin;
impl Plugin for XdgPopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Insert<XdgPopup>>();
        app.add_event::<Destroy<XdgPopup>>();
        app.register_type::<XdgPopup>();
    }
}
