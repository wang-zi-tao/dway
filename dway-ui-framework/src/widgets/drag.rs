use bevy::{ecs::relationship::Relationship as _, ui::RelativeCursorPosition};

use crate::{
    event::EventDispatcher,
    prelude::*,
    util::nodes::{get_node_position, set_node_position},
};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component, SmartDefault)]
    #[require(FocusPolicy=FocusPolicy::Block)]
    #[require(Node, RelativeCursorPosition, UiDragEventDispatcher)]
    pub struct UiDrag {
        pub moving: Option<
        pub struct UiDragState{
            pub pointer: Vec2,
            pub offset: Vec2,
        }>,
        #[default(true)]
        pub horizontal: bool,
        #[default(true)]
        pub vertical: bool,
        #[default(true)]
        pub auto_move: bool,
    }
}

#[derive(Debug, Clone)]
pub enum UiDragEvent {
    Start,
    Move { delta: Vec2, offset: Vec2 },
    End { offset: Vec2 },
}

pub type UiDragEventDispatcher = EventDispatcher<UiDragEvent>;

pub fn update_ui_drag(
    mut query: Query<
        (
            &ChildOf,
            &mut UiDrag,
            &RelativeCursorPosition,
            &Interaction,
            &UiDragEventDispatcher,
            &mut Node,
            &ComputedNode,
        ),
        Or<(Changed<Interaction>, Changed<RelativeCursorPosition>)>,
    >,
    parent_query: Query<&ComputedNode >,
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for (
        parent,
        mut this,
        relative_cursor_position,
        interaction,
        event_dispatcher,
        mut node,
        computed_node,
    ) in &mut query
    {
        if let Some(pointer) = get_node_mouse_position(relative_cursor_position, computed_node)
        {
            let UiDrag {
                moving,
                horizontal,
                vertical,
                auto_move,
            } = &mut *this;
            if let Some(state) = moving {
                if mouse.pressed(MouseButton::Left) {
                    if pointer != state.pointer {
                        let size = computed_node.size();
                        let parent_size =
                            if let Ok(parent_computed) = parent_query.get(parent.get()) {
                                parent_computed.size()
                            } else {
                                continue;
                            };

                        state.offset = pointer - state.pointer;
                        if !*horizontal {
                            state.offset.x = 0.0;
                        }
                        if !*vertical {
                            state.offset.y = 0.0;
                        }

                        let widget_offset = get_node_position(&node);
                        let mut new_offset = pointer - state.offset + widget_offset;

                        if *horizontal {
                            if new_offset.x < 0.0 {
                                new_offset.x = 0.0;
                            } else if new_offset.x + size.x > parent_size.x {
                                new_offset.x = parent_size.x - size.x;
                            }
                        }
                        if *vertical {
                            if new_offset.y < 0.0 {
                                new_offset.y = 0.0;
                            } else if new_offset.y + size.y > parent_size.y {
                                new_offset.y = parent_size.y - size.y;
                            }
                        }

                        event_dispatcher.send(
                            UiDragEvent::Move {
                                delta: state.offset,
                                offset: new_offset,
                            },
                            &mut commands,
                        );
                        if *auto_move {
                            set_node_position(&mut node, pointer);
                        }
                    }
                } else {
                    event_dispatcher.send(
                        UiDragEvent::End {
                            offset: state.pointer,
                        },
                        &mut commands,
                    );
                    this.moving = None;
                }
            } else if *interaction == Interaction::Pressed {
                event_dispatcher.send(UiDragEvent::Start, &mut commands);
                this.moving = Some(UiDragState {
                    pointer,
                    offset: Vec2::ZERO,
                });
            };
        }
    }
}
