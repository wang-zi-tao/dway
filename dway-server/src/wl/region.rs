use std::sync::Arc;

use crate::{prelude::*, util::rect::IRect};

#[derive(Resource)]
struct RegionDelegate(pub GlobalId);

#[derive(Debug,Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,FromReflect)]
pub enum RegionOperator {
    Add,
    Sub,
}
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlRegion {
    #[reflect(ignore)]
    pub raw: wl_region::WlRegion,
    pub rects: Vec<(RegionOperator, IRect)>,
    pub union: IRect,
}

impl WlRegion {
    pub fn new(raw: wl_region::WlRegion) -> Self {
        Self {
            raw,
            rects: vec![],
            union: Default::default(),
        }
    }
    pub fn add(&mut self, operator: RegionOperator, rect: IRect) {
        self.rects.push((operator, rect));
        if operator == RegionOperator::Add {
            self.union = self.union.union(rect);
        }
    }
    pub fn update_union(&mut self) {
        let mut union = IRect::default();
        for (operator, rect) in &self.rects {
            if *operator == RegionOperator::Add {
                union = union.union(*rect);
            }
        }
        self.union = union;
    }
    pub fn is_inside(&self, pos: IVec2) -> bool {
        let mut result = false;
        for (operator, rect) in &self.rects {
            if rect.include_point(pos) {
                match operator {
                    RegionOperator::Add => result = true,
                    RegionOperator::Sub => result = false,
                }
            }
        }
        result
    }
}

delegate_dispatch!(DWay: [wl_region::WlRegion: Entity] => RegionDelegate);
impl wayland_server::Dispatch<wl_region::WlRegion, bevy::prelude::Entity, DWay> for RegionDelegate {
    fn request(
        state: &mut DWay,
        client: &wayland_server::Client,
        resource: &wl_region::WlRegion,
        request: <wl_region::WlRegion as WlResource>::Request,
        data: &bevy::prelude::Entity,
        dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_region::Request::Destroy => {}
            wl_region::Request::Add {
                x,
                y,
                width,
                height,
            } => {
                state.with_component(resource, |c: &mut WlRegion| {
                    c.add(RegionOperator::Add, IRect::new(x, y, width, height))
                });
            }
            wl_region::Request::Subtract {
                x,
                y,
                width,
                height,
            } => todo!(),
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
impl wayland_server::GlobalDispatch<wayland_server::protocol::wl_region::WlRegion, ()> for DWay {
    fn bind(
        state: &mut Self,
        handle: &DisplayHandle,
        client: &wayland_server::Client,
        resource: wayland_server::New<wayland_server::protocol::wl_region::WlRegion>,
        global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        todo!()
    }
}

pub struct WlRegionPlugin(pub Arc<DisplayHandle>);
impl Plugin for WlRegionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RegionDelegate(
            self.0.create_global::<DWay, wl_region::WlRegion, ()>(1, ()),
        ));
        app.register_type::<WlRegion>();
    }
}
