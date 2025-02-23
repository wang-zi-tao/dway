
use bevy::{input::mouse::MouseWheel, ui::RelativeCursorPosition};

use crate::prelude::*;

#[derive(Component, SmartDefault, Reflect, Debug)]
#[require(Node, UiScrollState, UiScrollWidget, RelativeCursorPosition, Interaction)]
#[cfg_attr(feature = "hot_reload", derive(Serialize, Deserialize))]
pub struct UiScroll {
    pub horizontal: bool,
    #[default(true)]
    pub vertical: bool,
    pub create_viewport: bool,
}

dway_widget! {
UiScroll=>
@use_state(pub uv: Rect)
@use_state(pub offset: Vec2)
@use_state(pub size: Vec2)
@state_reflect()
@prop_reflect()
@arg(mut style_query:Query<(Ref<ComputedNode>,&mut Node)>)
@world_query(focus_police: &mut FocusPolicy)
@world_query(children: Option<&Children>)
@arg(mut mouse_wheel: EventReader<MouseWheel>)
@global(key_input: ButtonInput<KeyCode>)
@first{
    let mut wheel_move: Vec2 = mouse_wheel.read().map(|m|Vec2::new(m.x,m.y)).sum();
    if key_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        wheel_move = Vec2::new(wheel_move.y, wheel_move.x);
    }
}
@world_query(computed_node: &ComputedNode)
@world_query(transform: &GlobalTransform)
@world_query(mouse_position: Ref<RelativeCursorPosition>)
@use_state(pub content: Option<Entity>)
@before{
    if !widget.inited{
        if let Ok(( _,mut style )) = style_query.get_mut(this_entity){
            if prop.horizontal { style.overflow.x = OverflowAxis::Clip; }
            if prop.vertical { style.overflow.y = OverflowAxis::Clip; }
        }
        *focus_police = FocusPolicy::Block;
        if let Some(content) = children.and_then(|c|c.first()) {
            state.set_content(Some(*content));
        } else if prop.create_viewport {
            let content = commands.spawn(Node{
                min_width: Val::Percent(100.0),
                min_height: Val::Percent(100.0),
                ..Default::default()
            }).set_parent(this_entity).id();
        state.set_content(Some(content));
        }
    }
    (||{
        let scroll_rect = Rect::from_center_size(transform.translation().xy(), computed_node.size());
        let Some(content_entity) = *state.content() else {return};
        let Ok((content_node,mut content_style)) = style_query.get_mut(content_entity) else {return};
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
        Node=(Node{
            top: Val::Percent(state.uv().min.y*100.0),
            height: Val::Px(state.uv().size().y*state.size().y),
            ..style!("right-1 w-4 absolute")})
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("scroll-bar"), 4.0)) >
    </MiniNodeBundle>
</MiniNodeBundle>
<MiniNodeBundle @if(prop.horizontal) @style="absolute full">
    <MiniNodeBundle @id="vertical_handle"
        Node=(Node{
            left: Val::Percent(state.uv().min.x*100.0),
            width: Val::Px(state.uv().size().x*state.size().x),
            ..style!("bottom-1 h-4 absolute")})
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("scroll-bar"), 4.0)) >
    </MiniNodeBundle>
</MiniNodeBundle>
}
