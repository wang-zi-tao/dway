use bevy::prelude::{IVec2, Rect, Vec2};
use smithay::utils::{Point, Rectangle};

pub fn point_to_vec2<K>(point: Point<f64, K>) -> Vec2 {
    Vec2::new(point.x as f32, point.y as f32)
}
pub fn point_i32_to_vec2<K>(point: Point<i32, K>) -> Vec2 {
    Vec2::new(point.x as f32, point.y as f32)
}
pub fn point_to_ivec2<K>(point: Point<i32, K>) -> IVec2 {
    IVec2::new(point.x, point.y)
}
pub fn vec2_to_point<K>(vec: Vec2) -> Point<f32, K> {
    (vec.x as f32, vec.y as f32).into()
}
pub fn ivec2_to_point<K>(vec: IVec2) -> Point<i32, K> {
    (vec.x, vec.y).into()
}

pub fn rectangle_i32_to_rect<T>(rectangle: Rectangle<i32, T>) -> Rect {
    Rect::from_corners(
        point_i32_to_vec2(rectangle.loc),
        point_i32_to_vec2(rectangle.loc + rectangle.size),
    )
}
pub fn rectangle_to_rect<T>(rectangle: Rectangle<f64, T>) -> Rect {
    Rect::from_corners(
        point_to_vec2(rectangle.loc),
        point_to_vec2(rectangle.loc + rectangle.size),
    )
}
pub fn rect_to_rectangle<T>(rect: Rect) -> Rectangle<f32, T> {
    Rectangle::from_loc_and_size(vec2_to_point::<T>(rect.min), {
        let vec = rect.max - rect.min;
        (vec.x, vec.y)
    })
}
pub fn rectangle_i32_center<T>(rect: Rectangle<i32, T>) -> Point<i32, T> {
    Point::from((rect.loc.x + rect.size.w / 2, rect.loc.y + rect.size.h / 2))
}
