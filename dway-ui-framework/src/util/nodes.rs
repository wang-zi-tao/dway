use crate::prelude::*;

pub fn get_node_position(node: &Node) -> Vec2 {
    let x = match node.left {
        Val::Px(v) => v,
        _ => 0.0,
    };
    let y = match node.top {
        Val::Px(v) => v,
        _ => 0.0,
    };
    Vec2::new(x, y)
}

pub fn get_node_rect(node: &Node, compiluted_node: &ComputedNode) -> Rect {
    let pos = get_node_position(node);
    let size = compiluted_node.size();
    Rect::from_corners(pos, pos + size)
}

pub fn set_node_position(
    node: &mut Node,
    pos: Vec2,
) {
    node.left = Val::Px(pos.x);
    node.top = Val::Px(pos.y);
}
