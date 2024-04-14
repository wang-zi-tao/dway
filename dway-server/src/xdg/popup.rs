use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    resource::ResourceWrapper,
    util::rect::IRect,
    xdg::{positioner::XdgPositioner, DWayWindow},
};

use super::{positioner::Positioner, SurfaceHasPopup};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPopup {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_popup::XdgPopup,
    pub send_configure: bool,
    pub positioner: Positioner,
    pub level: isize,
    pub grab: bool,
}

impl XdgPopup {
    pub fn new(raw: xdg_popup::XdgPopup, positioner: Positioner, level: isize) -> Self {
        Self {
            raw,
            send_configure: false,
            positioner,
            level,
            grab: false,
        }
    }

    pub fn grab(&self) -> bool {
        self.grab
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
            xdg_popup::Request::Destroy => {}
            xdg_popup::Request::Grab { seat: _, serial: _ } => {
                if let Some(mut popup) = state.get_mut::<XdgPopup>(*data) {
                    popup.grab = true;
                }
            }
            xdg_popup::Request::Reposition {
                positioner,
                token: _,
            } => {
                let Some(positioner) =
                    state.with_component(&positioner, |c: &mut XdgPositioner| c.positioner.clone())
                else {
                    return;
                };
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
        resource: &xdg_popup::XdgPopup,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object_component::<(XdgPopup, DWayWindow)>(*data, resource);
        state.send_event(Destroy::<DWayWindow>::new(*data));
        state.disconnect_all_rev::<SurfaceHasPopup>(*data);
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
