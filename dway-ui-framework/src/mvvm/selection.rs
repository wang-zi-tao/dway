use std::{
    cmp::{max, min},
    collections::BTreeSet,
};

use bitflags::Flags;
use petgraph::csr::IndexType;

use super::IndexTrait;
use crate::prelude::*;

bitflags::bitflags! {
    pub struct SelectionState: u8 {
        const Selected = 1;
        const Focused = 1 << 1;
        const Pressed = 1 << 2;
    }
}

#[derive(Component)]
pub struct ItemSelectionInfo<Index: IndexTrait> {
    pub index: Index,
    pub view: Entity,
    pub state: SelectionState,
}

pub struct FocusModel<Index: IndexTrait> {
    pub focused: Option<(Index, Entity)>,
}

#[derive(Component)]
pub struct SelectionModel<Index: IndexTrait> {
    pub items: BTreeSet<(Index, Entity)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectMode {
    Single,
    AddByRange,
    Add,
    Toggle,
}

#[derive(Event)]
pub struct SelectItemRequest<Index: IndexTrait> {
    item_entity: Entity,
    container_entity: Entity,
    index: Index,
    mode: SelectMode,
}

#[derive(Component)]
pub struct DontSelect;

pub fn select_item_command(mut entity: EntityWorldMut) {
    let (ctrl, shift) = entity.world_scope(|world| {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        (ctrl, shift)
    });
}

pub fn update_selection<Index: IndexTrait>(
    mut item_query: Query<
        (Entity, &mut ItemSelectionInfo<Index>, &Interaction),
        (Without<DontSelect>, Changed<Interaction>),
    >,
    mut event_writer: EventWriter<SelectItemRequest<Index>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (entity, mut view_ref, interaction) in &mut item_query {
        let pressed = *interaction == Interaction::Pressed;
        let mouse_release = !pressed && view_ref.state.contains(SelectionState::Pressed);
        view_ref.state.set(SelectionState::Pressed, pressed);

        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let mode = if ctrl {
            SelectMode::Toggle
        } else if shift {
            SelectMode::AddByRange
        } else {
            SelectMode::Single
        };

        if mouse_release {
            event_writer.send(SelectItemRequest {
                item_entity: entity,
                container_entity: view_ref.view,
                index: view_ref.index.clone(),
                mode,
            });
        }
    }
}

pub fn do_select<Index: IndexTrait>(
    mut item_query: Query<&mut ItemSelectionInfo<Index>>,
    mut container: Query<&mut SelectionModel<Index>>,
    mut event_reader: EventReader<SelectItemRequest<Index>>,
) {
    for event in event_reader.read() {
        let mut set_selected = |entity: Entity, selected: bool| {
            let Ok(mut selection_info) = item_query.get_mut(event.item_entity) else {
                return;
            };
            selection_info.state.set(SelectionState::Selected, selected);
        };
        let Ok(mut selection_model) = container.get_mut(event.container_entity) else {
            continue;
        };
        let index = event.index.clone();
        let item_entity = event.item_entity;
        match event.mode {
            SelectMode::Single => {
                let items = BTreeSet::from_iter([(index, item_entity)]);
                let removed_items = std::mem::replace(&mut selection_model.items, items);
                for (_, removed_entity) in removed_items {
                    set_selected(removed_entity, false);
                }
                set_selected(event.item_entity, true);
            }
            SelectMode::AddByRange => {
                let item = (index, item_entity);
                let min = selection_model
                    .items
                    .first()
                    .map(|i| min(i, &item))
                    .unwrap_or(&item);
                let max = selection_model
                    .items
                    .last()
                    .map(|i| max(i, &item))
                    .unwrap_or(&item);
                todo!();
            }
            SelectMode::Add => {
                let added = selection_model.items.insert((index, item_entity));
                if added {
                    set_selected(event.item_entity, true);
                }
            }
            SelectMode::Toggle => {
                let item = (index, item_entity);
                if selection_model.items.contains(&item) {
                    selection_model.items.remove(&item);
                    set_selected(event.item_entity, false);
                } else {
                    selection_model.items.insert(item);
                    set_selected(event.item_entity, true);
                }
            }
        }
    }
}
