use bevy::{input::mouse::MouseWheel, ui::RelativeCursorPosition};

use crate::prelude::*;

#[derive(Component, SmartDefault, Reflect, Debug)]
#[cfg_attr(feature = "hot_reload", derive(Serialize, Deserialize))]
pub struct UiScroll {
    pub horizontal: bool,
    #[default(true)]
    pub vertical: bool,
}

dway_widget! {
UiScroll=>
@use_state(uv: Rect)
@use_state(offset: Vec2)
@use_state(size: Vec2)
@state_reflect()
@prop_reflect()
@arg(mut style_query:Query<(Ref<Node>,&mut Style)>)
@world_query(focus_police: &mut FocusPolicy)
@world_query(children: &Children)
@arg(mut mouse_wheel: EventReader<MouseWheel>)
@global(key_input: ButtonInput<KeyCode>)
@first{
    let mut wheel_move: Vec2 = mouse_wheel.read().map(|m|Vec2::new(m.x,m.y)).sum();
    if key_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        wheel_move = Vec2::new(wheel_move.y, wheel_move.x);
    }
}
@bundle({
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy = FocusPolicy::Block,
    pub cursor_positon: RelativeCursorPosition, // TODO 优化
})
@world_query(node: &Node)
@world_query(transform: &GlobalTransform)
@world_query(mouse_position: Ref<RelativeCursorPosition>)
@use_state(pub content: Entity = Entity::PLACEHOLDER)
@before{
    if !widget.inited{
        if let Ok(( _,mut style )) = style_query.get_mut(this_entity){
            if prop.horizontal { style.overflow.x = OverflowAxis::Clip; }
            if prop.vertical { style.overflow.y = OverflowAxis::Clip; }
        }
        *focus_police = FocusPolicy::Block;
        if let Some(content) = children.first() {
            state.set_content(*content);
        }
    }
    (||{
        let scroll_rect = Rect::from_center_size(transform.translation().xy(), node.size());
        let Ok((content_node,mut content_style)) = style_query.get_mut(*state.content()) else {return};
        let inside = mouse_position.mouse_over();
        if !content_node.is_changed() && wheel_move == Vec2::ZERO && !inside {return};
        let diff_size = content_node.size() - scroll_rect.size();
        let offset = if diff_size.x<0.0 && prop.horizontal || diff_size.y<0.0 && prop.vertical || !inside {
            *state.offset()
        } else {
            let mut wheel_move = wheel_move;
            if !prop.horizontal {wheel_move.x=0.0;}
            if !prop.vertical {wheel_move.y=0.0;}
            *state.offset() - wheel_move * 64.0
        };
        let offset = offset.max(Vec2::ZERO).min(diff_size);
        if *state.size() != scroll_rect.size() {
            state.set_size(scroll_rect.size());
        }
        if offset != *state.offset() {
            state.set_offset(offset);
            state.set_uv(Rect::from_corners(offset/content_node.size(), (offset + scroll_rect.size())/content_node.size()));
            content_style.left = Val::Px(-offset.x);
            content_style.top = Val::Px(-offset.y);
        }
    })();
}
@global(theme:Theme)
<MiniNodeBundle @if(prop.vertical) @style="absolute full">
    <MiniNodeBundle @id="vertical_handle"
        Style=(Style{
            top: Val::Percent(state.uv().min.y*100.0),
            height: Val::Px(state.uv().size().y*state.size().y),
            ..style!("right-1 w-4 absolute")})
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("scroll-bar"), 4.0)) >
    </MiniNodeBundle>
</MiniNodeBundle>
<MiniNodeBundle @if(prop.horizontal) @style="absolute full">
    <MiniNodeBundle @id="vertical_handle"
        Style=(Style{
            left: Val::Percent(state.uv().min.x*100.0),
            width: Val::Px(state.uv().size().x*state.size().x),
            ..style!("bottom-1 h-4 absolute")})
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("scroll-bar"), 4.0)) >
    </MiniNodeBundle>
</MiniNodeBundle>
}
