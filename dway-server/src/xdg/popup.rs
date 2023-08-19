use crate::{
    geometry::{Geometry, GlobalGeometry},
    input::{
        grab::Grab,
        seat::{PointerList, WlSeat},
    },
    prelude::*,
    resource::ResourceWrapper,
    util::rect::IRect,
    xdg::{positioner::XdgPositioner, DWayWindow},
};

use super::positioner::Positioner;

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
        _client: &wayland_server::Client,
        resource: &xdg_popup::XdgPopup,
        request: <xdg_popup::XdgPopup as wayland_server::Resource>::Request,
        data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
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
                let Some(_pointer_list) = state.world_mut().get::<PointerList>(seat_entity).cloned()
                else {
                    return;
                };
                let Some(parent_entity) = state.get::<Parent>(*data).map(|p| p.get())
                else {
                    return;
                };
                let parent_is_popup = state.entity(parent_entity).contains::<XdgPopup>();
                state.query::<(&mut Grab, &mut WlSeat), _, _>(
                    DWay::get_entity(&seat),
                    |(mut grab, mut seat)| {
                        if let Grab::OnPopup {
                            surface_entity,
                            popup_stack,
                            pressed: _,
                            serial: _,
                        } = &mut *grab
                        {
                            dbg!(parent_entity, parent_is_popup);
                            if parent_is_popup {
                                let index =
                                    popup_stack.iter().rev().enumerate().find(|(_index, popup)| {
                                        DWay::get_entity(*popup) == parent_entity
                                    });
                                dbg!(index);
                                if let Some((index, _)) = index {
                                    if index + 1 != popup_stack.len() {
                                        popup_stack.drain(index + 1..).for_each(
                                            |popup| {
                                                if popup.is_alive() {
                                                    popup.popup_done()
                                                }
                                            },
                                        );
                                    }
                                    popup_stack.push(resource.clone());
                                    *surface_entity = *data;
                                    return;
                                } else {
                                    warn!("failed to grab popup, parent popup is not grabed");
                                }
                            }
                        }
                        *grab = Grab::OnPopup {
                            surface_entity: *data,
                            popup_stack: vec![resource.clone()],
                            pressed: false,
                            serial,
                        };
                        seat.unset_grab();
                    },
                );
            }
            xdg_popup::Request::Reposition { positioner, token: _ } => {
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
