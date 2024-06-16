use std::{marker::PhantomData, ops::Range};

use crate::prelude::*;

use super::{
    ContainerViewFactory, ContainerViewModel, DataItem, DynEntityCommand, EntityWorldRef,
    ViewFactory, ViewItem, ViewItemState,
};

#[bevy_trait_query::queryable]
pub trait ListViewModel<Item: DataItem>: ContainerViewModel<usize, Item> {
    fn len(&self, world: &World) -> usize;
}

#[bevy_trait_query::queryable]
pub trait ListItemViewFactory<Item: DataItem>: ContainerViewFactory<usize, Item> {}

#[derive(Component)]
pub struct List<Item: DataItem> {
    pub phantom: PhantomData<Item>,
}

impl<Item: DataItem> List<Item> {
    pub fn update_system(
        world: &World,
        mut query: Query<
            (
                Entity,
                &Children,
                One<&dyn ListViewModel<Item>>,
                One<&dyn ListItemViewFactory<Item>>,
            ),
            With<Self>,
        >,
        mut item_query: Query<&mut ViewItem<Item>>,
        mut commands: Commands,
    ) {
        for (entity, children, model, view) in &mut query {
            let entity_ref = EntityWorldRef::new(world, entity);
            if model.update_from_world(entity_ref) {
                let size = model.len(world);
                if size < children.len() {
                    commands.entity(entity).remove_children(&children[size..]);
                    for child in &children[size..] {
                        commands.entity(*child).despawn_recursive();
                    }
                }
                for i in 0..size {
                    let Some(item) = model.get(&i, entity_ref) else {
                        continue;
                    };
                    if let Some(mut item_container) = children
                        .get(i)
                        .and_then(|child| item_query.get_mut(*child).ok())
                    {
                        item_container.set_item(item);
                    } else {
                        view.create(&i, commands.entity(entity), item);
                    }
                }
            }
        }
    }
}

pub struct ListViewPlugin<Item: DataItem>(PhantomData<Item>);
impl<Item: DataItem> Plugin for ListViewPlugin<Item> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            List::<Item>::update_system.in_set(UiFrameworkSystems::UpdateMVVM),
        );
    }
}
