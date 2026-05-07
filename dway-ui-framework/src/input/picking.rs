use crate::prelude::*;

#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[require(UiInput)]
#[component(on_insert=on_insert_picking_input)]
struct UiPickingInput {
    pub hovered: bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub middle_pressed: bool,
}

impl UiPickingInput {
    pub fn is_any_button_pressed(&self) -> bool {
        self.left_pressed || self.right_pressed || self.middle_pressed
    }
}

fn on_insert_picking_input(mut world: DeferredWorld, context: HookContext) {
    world
        .commands()
        .entity(context.entity)
        .queue(|mut entity: EntityWorldMut| {
            let id = entity.id();
            entity.world_scope(|world| {
                world.resource_scope::<CallbackTypeRegister, _>(|world, mut callbacks| {
                    callbacks.add_to_observer_in_world(on_drag_start, id, world);
                    callbacks.add_to_observer_in_world(on_drag, id, world);
                    callbacks.add_to_observer_in_world(on_drag_end, id, world);
                    callbacks.add_to_observer_in_world(on_drag_enter, id, world);
                    callbacks.add_to_observer_in_world(on_drag_over, id, world);
                    callbacks.add_to_observer_in_world(on_drag_leave, id, world);
                    callbacks.add_to_observer_in_world(on_drag_drop, id, world);
                    callbacks.add_to_observer_in_world(on_drag_entry, id, world);
                })
            })
        });
}

macro_rules! picking_callback {
    ($name:ident, $event:ident) => {
        fn $name(
            event: On<Pointer<$event>>,
            query: Query<&UiInputEventDispatcher>,
            mut commands: Commands,
        ) {
            let Ok(input_dispatcher) = query.get(event.entity) else {
                return;
            };

            input_dispatcher.send(
                UiInputEvent::$event(event.event().event.clone()),
                &mut commands,
            );
        }
    };
}

picking_callback!(on_drag_start, DragStart);
picking_callback!(on_drag, Drag);
picking_callback!(on_drag_end, DragEnd);
picking_callback!(on_drag_enter, DragEnter);
picking_callback!(on_drag_over, DragOver);
picking_callback!(on_drag_leave, DragLeave);
picking_callback!(on_drag_drop, DragDrop);
picking_callback!(on_drag_entry, DragEntry);
