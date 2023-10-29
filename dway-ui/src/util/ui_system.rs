use std::marker::PhantomData;

use bevy::{
    ecs::{
        archetype::ArchetypeComponentId,
        component::{ComponentId, Tick},
        query::{Access, WorldQuery},
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
};

#[derive(Component)]
pub struct TestProp(String);
#[derive(Component)]
pub struct TestState(String);

#[derive(Component)]
pub struct TestWidgets {
    test0: Entity,
}

fn test_render(In((entity, prop, commands)): In<(Entity, &TestProp, &mut Commands)>) {
    let mut test0_entity = Entity::PLACEHOLDER;
    commands
        .entity(entity)
        .with_children(|c| {
            test0_entity = c
                .spawn(TextBundle {
                    text: Text::from_section(
                        prop.0.clone(),
                        TextStyle {
                            font_size: 60.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    ..default()
                })
                .id();
        })
        .insert(TestWidgets {
            test0: test0_entity,
        });
}

fn widget_system(
    prop_query: Query<(Entity, &TestProp), Changed<TestProp>>,
    mut commands: &mut Commands,
) {
    prop_query.for_each(|(entity, prop)| {
        test_render(In((entity, prop, commands)));
    });
}

pub struct ParenProp(String);
fn parent_test_render(In((entity, prop, commands)): In<(Entity, &ParenProp, &mut Commands)>) {
    commands.entity(entity).with_children(|c| {
        c.spawn(TestProp(prop.0.clone()));
    });
}
