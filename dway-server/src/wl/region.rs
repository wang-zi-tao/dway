use crate::{prelude::*, util::rect::IRect};
use rstar::{PointDistance, RTree, RTreeObject, SelectionFunction, AABB};

#[derive(Resource)]
struct RegionDelegate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
pub enum RegionOperator {
    Add,
    Sub,
}
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Debug)]
pub struct WlRegion {
    #[reflect(ignore, default = "unimplemented")]
    pub raw: wl_region::WlRegion,
    pub rects: Vec<(RegionOperator, IRect)>,
    pub union: IRect,
}

#[derive(Bundle)]
pub struct WlRegionBundle {
    pub wl_region: WlRegion,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RectRTreeObject {
    pub rect: IRect,
    pub operator: RegionOperator,
    pub index: usize,
    pub entity: Entity,
}
impl RTreeObject for RectRTreeObject {
    type Envelope = AABB<[i32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.rect.min.x, self.rect.min.y],
            [self.rect.max.x, self.rect.max.y],
        )
    }
}
impl PointDistance for RectRTreeObject {
    fn distance_2(
        &self,
        point: &<Self::Envelope as rstar::Envelope>::Point,
    ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
        self.envelope().distance_2(point)
    }
}

#[derive(Event)]
pub struct RectAddEvent(pub RectRTreeObject);

#[derive(Event)]
pub struct RectRemoveEvent(pub RectRTreeObject);

#[derive(Event)]
pub struct RectRemoveAllEvent(pub Entity);

#[derive(Resource, Component)]
pub struct RTreeIndex {
    pub rtree: RTree<RectRTreeObject>,
}
impl RTreeIndex {
    pub fn find_all(&self, position: IVec2) {
        self.rtree.locate_all_at_point(&[position.x, position.y]);
    }
}

pub fn update_region_index(
    mut index: ResMut<RTreeIndex>,
    mut add_event: EventReader<RectAddEvent>,
    mut remove_event: EventReader<RectRemoveEvent>,
    mut remove_entity_event: EventReader<RectRemoveAllEvent>,
) {
    for RectAddEvent(rect) in add_event.read() {
        index.rtree.insert(rect.clone());
    }
    for RectRemoveEvent(rect) in remove_event.read() {
        index.rtree.remove(rect);
    }
    struct Selection(pub Entity);
    impl SelectionFunction<RectRTreeObject> for Selection {
        fn should_unpack_parent(
            &self,
            _envelope: &<RectRTreeObject as RTreeObject>::Envelope,
        ) -> bool {
            true
        }

        fn should_unpack_leaf(&self, leaf: &RectRTreeObject) -> bool {
            leaf.entity == self.0
        }
    }
    for RectRemoveAllEvent(entity) in remove_entity_event.read() {
        index
            .rtree
            .remove_with_selection_function(Selection(*entity));
    }
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
        _client: &wayland_server::Client,
        resource: &wl_region::WlRegion,
        request: <wl_region::WlRegion as WlResource>::Request,
        _data: &bevy::prelude::Entity,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, DWay>,
    ) {
        match request {
            wl_region::Request::Destroy => {
                state.destroy_object(resource);
            }
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
            } => {
                state.with_component(resource, |c: &mut WlRegion| {
                    c.add(RegionOperator::Sub, IRect::new(x, y, width, height))
                });
            }
            _ => todo!(),
        }
    }
    fn destroyed(
        state: &mut DWay,
        _client: wayland_backend::server::ClientId,
        resource: &wl_region::WlRegion,
        data: &bevy::prelude::Entity,
    ) {
        state.despawn_object(*data, resource);
    }
}
pub struct WlRegionPlugin;
impl Plugin for WlRegionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WlRegion>();
    }
}
