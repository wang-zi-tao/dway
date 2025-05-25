use wayland_protocols::xdg::shell::server::xdg_positioner::{Anchor, ConstraintAdjustment, Gravity};

use crate::{
    geometry::{Geometry, GlobalGeometry},
    prelude::*,
    util::rect::IRect,
};

#[derive(Default, Reflect, Debug, Clone)]
pub struct Positioner {
    pub anchor_rect: Option<IRect>,
    #[reflect(ignore)]
    pub constraint_adjustment: Option<WEnum<ConstraintAdjustment>>,
    #[reflect(ignore)]
    pub anchor_kind: Option<Anchor>,
    #[reflect(ignore)]
    pub gravity: Option<Gravity>,
    pub is_relative: bool,
    pub parent_size: Option<IVec2>,
    pub offset: Option<IVec2>,
    pub size: Option<IVec2>,
}

impl Positioner {
    pub fn get_geometry(&self) -> IRect {
        let size = self.size.unwrap_or_default();
        let mut geometry = IRect::from_pos_size(self.offset.unwrap_or_default(), size);

        let anchor_rect = self.anchor_rect.unwrap_or_default();
        match self.anchor_kind {
            Some(xdg_positioner::Anchor::TopLeft)
            | Some(xdg_positioner::Anchor::TopRight)
            | Some(xdg_positioner::Anchor::Top) => {
                geometry.set_y(geometry.y() + anchor_rect.y());
            }
            Some(xdg_positioner::Anchor::BottomLeft)
            | Some(xdg_positioner::Anchor::BottomRight)
            | Some(xdg_positioner::Anchor::Bottom) => {
                geometry.set_y(geometry.y() + anchor_rect.max.y);
            }
            _ => {
                geometry.set_y(geometry.y() + anchor_rect.center().y);
            }
        }

        match self.anchor_kind {
            Some(xdg_positioner::Anchor::TopLeft)
            | Some(xdg_positioner::Anchor::BottomLeft)
            | Some(xdg_positioner::Anchor::Left) => {
                geometry.set_x(geometry.x() + anchor_rect.x());
            }
            Some(xdg_positioner::Anchor::BottomRight)
            | Some(xdg_positioner::Anchor::TopRight)
            | Some(xdg_positioner::Anchor::Right) => {
                geometry.set_x(geometry.x() + anchor_rect.max.x);
            }
            _ => {
                geometry.set_x(geometry.x() + anchor_rect.center().x);
            }
        }

        match self.gravity {
            Some(xdg_positioner::Gravity::TopLeft)
            | Some(xdg_positioner::Gravity::TopRight)
            | Some(xdg_positioner::Gravity::Top) => {
                geometry.set_y(geometry.y() - size.y);
            }
            Some(xdg_positioner::Gravity::BottomLeft)
            | Some(xdg_positioner::Gravity::BottomRight)
            | Some(xdg_positioner::Gravity::Bottom) => {}
            _ => {
                geometry.set_y(geometry.y() - size.y / 2);
            }
        }

        match self.gravity {
            Some(xdg_positioner::Gravity::TopLeft)
            | Some(xdg_positioner::Gravity::BottomLeft)
            | Some(xdg_positioner::Gravity::Left) => {
                geometry.set_x(geometry.x() - size.x);
            }
            Some(xdg_positioner::Gravity::BottomRight)
            | Some(xdg_positioner::Gravity::TopRight)
            | Some(xdg_positioner::Gravity::Right) => {}
            _ => {
                geometry.set_x(geometry.x() - size.x / 2);
            }
        }

        geometry
    }
}

#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct XdgPositioner {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: xdg_positioner::XdgPositioner,
    pub positioner: Positioner,
}
impl XdgPositioner {
    pub fn new(raw: xdg_positioner::XdgPositioner) -> Self {
        Self {
            raw,
            positioner: Default::default(),
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
        _client: &wayland_server::Client,
        resource: &xdg_positioner::XdgPositioner,
        request: <xdg_positioner::XdgPositioner as WlResource>::Request,
        data: &Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let span =
            span!(Level::ERROR,"request",entity = ?data,resource = %WlResource::id(resource));
        let _enter = span.enter();
        debug!("request {:?}", &request);
        match request {
            xdg_positioner::Request::Destroy => {
                state.destroy_object(resource);
            }
            xdg_positioner::Request::SetSize { width, height } => {
                state.with_component(resource, |c: &mut Geometry| {
                    c.set_size(IVec2::new(width, height));
                });
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.positioner.size = Some(IVec2::new(width, height));
                });
            }
            xdg_positioner::Request::SetAnchorRect {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.positioner.anchor_rect = Some(IRect::new(x, y, width, height));
                });
            }
            xdg_positioner::Request::SetAnchor { anchor } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    DWay::set_enum(anchor, |e| c.positioner.anchor_kind = Some(e));
                });
            }
            xdg_positioner::Request::SetGravity { gravity } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    DWay::set_enum(gravity, |e| c.positioner.gravity = Some(e));
                });
            }
            xdg_positioner::Request::SetConstraintAdjustment {
                constraint_adjustment,
            } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.positioner.constraint_adjustment = Some(constraint_adjustment);
                });
            }
            xdg_positioner::Request::SetOffset { x, y } => {
                state.query::<(&mut Geometry, &mut XdgPositioner), _, _>(
                    *data,
                    |(mut g, mut p)| {
                        g.set_pos(IVec2::new(x, y));
                        p.positioner.offset = Some(IVec2::new(x, y));
                    },
                );
            }
            xdg_positioner::Request::SetReactive => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.positioner.is_relative = true;
                });
            }
            xdg_positioner::Request::SetParentSize {
                parent_width,
                parent_height,
            } => {
                state.with_component(resource, |c: &mut XdgPositioner| {
                    c.positioner.parent_size = Some(IVec2::new(parent_width, parent_height));
                });
            }
            xdg_positioner::Request::SetParentConfigure { serial: _ } => {}
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &xdg_positioner::XdgPositioner,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
