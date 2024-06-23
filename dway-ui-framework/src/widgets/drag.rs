use bevy::ui::RelativeCursorPosition;
use bevy_relationship::reexport::SmallVec;

use crate::{event::EventDispatch, make_bundle, prelude::*};

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    #[derive(Component, Default)]
    pub struct UiDrag {
        pub state: Option<
        pub struct UiDragState{
            pub delta: Vec2,
            pub move_vec: Vec2,
        }>,
        #[reflect(ignore)]
        pub callbacks: SmallVec<[(Entity, SystemId<UiDragEvent>); 2]>,
    }
}

impl UiDrag {
    pub fn with_callback(mut self, receiver: Entity, systemid: SystemId<UiDragEvent>) -> Self {
        self.callbacks.push((receiver, systemid));
        self
    }
}

structstruck::strike! {
    #[strikethrough[derive(Debug, Clone)]]
    pub struct UiDragEvent{
        pub ui_drag: Entity,
        pub receiver: Entity,
        pub kind: pub enum UiDragEventKind{
            Start,
            Move(Vec2),
            End,
        }
    }
}

pub fn update_ui_drag(
    mut query: Query<
        (
            Entity,
            &mut UiDrag,
            &Node,
            &RelativeCursorPosition,
            &Interaction,
            Option<&dyn EventDispatch<UiDragEvent>>,
        ),
        Or<(Changed<Interaction>, Changed<RelativeCursorPosition>)>,
    >,
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    for (entity, mut this, node, cursor, interaction, dispatchs) in &mut query {
        let mut send_event = |kind: UiDragEventKind, this: &mut UiDrag| {
            for (receiver, callback) in &this.callbacks {
                commands.run_system_with_input(
                    *callback,
                    UiDragEvent {
                        ui_drag: entity,
                        receiver: *receiver,
                        kind: kind.clone(),
                    },
                );
            }
            let mut entity_commands = commands.entity(entity);
            for dispatch in dispatchs.iter().flatten() {
                dispatch.on_event(
                    entity_commands.reborrow(),
                    UiDragEvent {
                        ui_drag: entity,
                        receiver: entity,
                        kind: kind.clone(),
                    },
                );
            }
        };
        if let Some(normalized_cursor_pos) = cursor.normalized {
            if let Some(state) = &mut this.state {
                if mouse.pressed(MouseButton::Left) {
                    let pos = normalized_cursor_pos * node.size();
                    if pos != state.delta {
                        state.move_vec = pos - state.delta;
                        send_event(UiDragEventKind::Move(pos - state.delta), &mut this);
                    }
                } else {
                    send_event(UiDragEventKind::End, &mut this);
                    this.state = None;
                }
            } else if *interaction == Interaction::Pressed {
                send_event(UiDragEventKind::Start, &mut this);
                this.state = Some(UiDragState {
                    delta: normalized_cursor_pos * node.size(),
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
    }
}
