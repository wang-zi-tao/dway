use bevy_math::{IVec2, Rect, Vec2};
use smithay::utils::{Point, Rectangle};

pub fn point_to_vec2<K>(point: Point<f64, K>) -> Vec2 {
    Vec2::new(point.x as f32, point.y as f32)
}
pub fn point_to_ivec2<K>(point: Point<i32, K>) -> IVec2 {
    IVec2::new(point.x, point.y)
}
pub fn vec2_to_point<K>(vec: Vec2) -> Point<f64, K> {
    (vec.x as f64, vec.y as f64).into()
}

pub fn rectangle_to_rect<T>(rectangle: Rectangle<f64, T>) -> Rect {
    Rect::from_corners(
        point_to_vec2(rectangle.loc),
        point_to_vec2(rectangle.loc + rectangle.size),
    )
}
pub fn rect_to_rectangle<T>(rect: Rect) -> Rectangle<f64, T> {
    Rectangle::from_loc_and_size(vec2_to_point::<T>(rect.min), {
        let vec = rect.max - rect.min;
        (vec.x as f64, vec.y as f64)
    })
}
