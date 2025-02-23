use bevy::{
    app::ScheduleRunnerPlugin,
    ecs::{system::BoxedSystem, world::Command},
    prelude::*,
    utils::HashSet,
};
use bevy_relationship::{
    relationship, AppExt, ConnectCommand, Connectable, ControlFlow, EntityCommandsExt, SharedReferenceFrom, SharedReferenceRelationship, UniqueReferenceRelationship
};
use bevy_relationship_derive::graph_query2;

relationship!(R0=>F0--T0);

#[test]
fn test_disconnect() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));

    let e0 = app.world_mut().spawn_empty().id();
    let e1 = app.world_mut().spawn_empty().id();
    ConnectCommand::<R0>::new(e0, e1).apply(app.world_mut());
    assert_eq!(app.world().get::<F0>(e0).unwrap().as_slice(), &[e1]);
    assert_eq!(app.world().get::<T0>(e1).unwrap().as_slice(), &[e0]);

    app.world_mut().despawn(e0);
    assert_eq!(app.world().get::<T0>(e1).unwrap().iter().len(), 0);

    app.run();
}

#[test]
fn test_unique_ref() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));

    let e0 = app.world_mut().spawn_empty().id();
    let e1 = app.world_mut().spawn_empty().id();
    ConnectCommand::<UniqueReferenceRelationship>::new(e0, e1).apply(app.world_mut());

    app.world_mut().despawn(e0);
    assert!(app.world().get_entity(e1).is_err());

    app.run();
}

#[test]
fn test_shared_ref() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()));

    let e0 = app.world_mut().spawn_empty().id();
    let e1 = app.world_mut().spawn_empty().id();
    let e2 = app.world_mut().spawn_empty().id();
    ConnectCommand::<SharedReferenceRelationship>::new(e0, e2).apply(app.world_mut());
    ConnectCommand::<SharedReferenceRelationship>::new(e1, e2).apply(app.world_mut());

    assert_eq!(app.world().get::<SharedReferenceFrom>(e2).unwrap().iter().len(), 2);
    app.world_mut().despawn(e0);
    assert_eq!(app.world().get::<SharedReferenceFrom>(e2).unwrap().iter().len(), 1);
    app.world_mut().despawn(e1);
    assert!(app.world().get_entity(e2).is_err());

    app.run();
}
