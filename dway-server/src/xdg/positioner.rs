use wayland_protocols::xdg::shell::server::xdg_positioner::{Anchor, Gravity};

use crate::{
    create_dispatch,
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    util::rect::IRect,
};

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPositioner {
    #[reflect(ignore)]
    pub raw: xdg_positioner::XdgPositioner,
    pub anchor_rect: Option<IRect>,
    pub constraint_adjustment: Option<u32>,
    #[reflect(ignore)]
    pub anchor_kind: Option<Anchor>,
    #[reflect(ignore)]
    pub gravity: Option<Gravity>,
    pub is_relative: bool,
}
impl XdgPositioner {
    pub fn new(raw: xdg_positioner::XdgPositioner) -> Self {
        Self {
            raw,
            anchor_rect: Default::default(),
            anchor_kind: None,
            gravity: None,
            constraint_adjustment: None,
            is_relative: false,
        }
    }
}
#[derive(Bundle)]
pub struct XdgPositionerBundle {
    pub resource: XdgPositioner,
    pub geo: Geometry,
    pub rect: GlobalGeometry,
}
impl XdgPositionerBundle {
    pub fn new(resource: XdgPositioner) -> Self {
        Self {
            resource,
            geo: Default::default(),
            rect: Default::default(),
        }
    }
}
impl Dispatch<xdg_positioner::XdgPositioner, Entity> for DWay {
    fn request(
        state: &mut Self,
        client: &wayland_server::Client,
        resource: &xdg_positioner::XdgPositioner,
        request: <xdg_positioner::XdgPositioner as WlResource>::Request,
        data: &Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_positioner::Request::Destroy => {
                state.destroy_object::<XdgPositioner>(resource);
            }
            xdg_positioner::Request::SetSize { width, height } => {
                state.with_component(resource, |c: &mut Geometry| {
                    c.set_size(IVec2::new(width, height));
                });
            }
            xdg_positioner::Request::SetAnchorRect {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.anchor_rect = Some(IRect::new(x, y, width, height));
                });
            }
            xdg_positioner::Request::SetAnchor { anchor } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    DWay::set_enum(anchor, |e| c.anchor_kind = Some(e));
                });
            }
            xdg_positioner::Request::SetGravity { gravity } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    DWay::set_enum(gravity, |e| c.gravity = Some(e));
                });
            }
            xdg_positioner::Request::SetConstraintAdjustment {
                constraint_adjustment,
            } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.constraint_adjustment = Some(constraint_adjustment);
                });
            }
            xdg_positioner::Request::SetOffset { x, y } => {
                state.with_component(resource, |c: &mut Geometry| {
                    c.set_pos(IVec2::new(x, y));
                });
            }
            xdg_positioner::Request::SetReactive => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.is_relative = true;
                });
            }
            xdg_positioner::Request::SetParentSize {
                parent_width,
                parent_height,
            } => todo!(),
            xdg_positioner::Request::SetParentConfigure { serial } => todo!(),
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
