use bevy::ui::RelativeCursorPosition;

use crate::{
    event::EventDispatcher,
    make_bundle,
    prelude::*,
};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component, Default)]
    pub struct UiDrag {
        pub state: Option<
        pub struct UiDragState{
            pub delta: Vec2,
            pub move_vec: Vec2,
        }>,
    }
}

#[derive(Debug, Clone)]
pub enum UiDragEvent {
    Start,
    Move(Vec2),
    End,
}

pub type UiDragEventDispatcher = EventDispatcher<UiDragEvent>;

pub fn update_ui_drag(
    mut query: Query<
        (
            Entity,
            &mut UiDrag,
            &ComputedNode,
            &RelativeCursorPosition,
            &Interaction,
            &UiDragEventDispatcher,
        ),
        Or<(Changed<Interaction>, Changed<RelativeCursorPosition>)>,
    >,
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for (entity, mut this, computed_node, cursor, interaction, event_dispatcher) in &mut query {
        if let Some(normalized_cursor_pos) = cursor.normalized {
            if let Some(state) = &mut this.state {
                if mouse.pressed(MouseButton::Left) {
                    let pos = normalized_cursor_pos * computed_node.size();
                    if pos != state.delta {
                        state.move_vec = pos - state.delta;
                        event_dispatcher.send(UiDragEvent::Move(pos - state.delta), &mut commands);
                    }
                } else {
                    event_dispatcher.send(UiDragEvent::End, &mut commands);
                    this.state = None;
                }
            } else if *interaction == Interaction::Pressed {
                event_dispatcher.send(UiDragEvent::Start, &mut commands);
                this.state = Some(UiDragState {
                    delta: normalized_cursor_pos * computed_node.size(),
                    move_vec: Vec2::ZERO,
                });
            };
        }
    }
}

make_bundle! {
    @from drag: UiDrag,
    @addon UiDragExt,
    UiDragBundle{
        pub drag: UiDrag,
        pub interaction: Interaction,
        #[default(FocusPolicy::Block)]
        pub focus_policy: FocusPolicy,
        pub relative_cursor_position: RelativeCursorPosition,
        pub event_dispatcher: UiDragEventDispatcher,
    }
}
