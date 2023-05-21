use bevy::prelude::*;
use bevy_prototype_lyon::shapes;

pub fn generate_window_shape(rect: Rect, is_ssd: bool) {
    let shape = shapes::RegularPolygon {
        sides: 6,
        feature: shapes::RegularPolygonFeature::Radius(200.0),
        ..shapes::RegularPolygon::default()
    };
    let shape = shapes::RoundedPolygon {
        points: todo!(),
        radius: todo!(),
        closed: todo!(),
    };
}
