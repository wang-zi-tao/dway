#![feature(test)]
extern crate test;
use std::sync::Mutex;

use bevy::prelude::*;
use bevy_relationship::{
    DefaultUpdateGraphRegistery, ExecutorKind, TriggerPlugin, TriggerScheduleLabel, UpdateEntitys,
    UpdateEvent,
};
#[derive(Component, Clone, Copy)]
pub struct C0;

#[derive(Component)]
pub struct C1;

#[derive(Component)]
pub struct C2;

#[derive(Resource, Default)]
pub struct ResultContainer {
    pub results: Mutex<Vec<String>>,
}
impl ResultContainer {
    pub fn push(&self, value: &str) {
        self.results.lock().unwrap().push(value.to_string());
    }

    pub fn get(&mut self) -> &[String] {
        self.results.get_mut().unwrap().as_slice()
    }
}

pub fn run_test(
    executor_kind: ExecutorKind,
    init: impl FnOnce(&mut World),
    check: impl FnOnce(&mut World),
) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(TriggerPlugin::<TriggerScheduleLabel> {
        executor_kind,
        ..default()
    });
    app.init_resource::<ResultContainer>();
    init(app.world_mut());
    app.finish();

    app.update();
    check(app.world_mut());
}

#[test]
pub fn sinsgle_system() {
    run_test(
        ExecutorKind::SingleThread,
        |world| {
            let component0 = world.init_component::<C0>();
            let entity0 = world.spawn(C0).id();
            world.send_event(UpdateEvent::new(entity0, component0));

            DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );
        },
        |world| {
            let mut results = world.resource_mut::<ResultContainer>();
            assert_eq!(results.get(), &["run".to_string()]);
        },
    );
}

#[test]
pub fn single_component_many_system() {
    run_test(
        ExecutorKind::SingleThread,
        |world| {
            let component0 = world.init_component::<C0>();
            let entity0 = world.spawn(C0).id();
            world.send_event(UpdateEvent::new(entity0, component0));

            DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );

            DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );
        },
        |world| {
            let mut results = world.resource_mut::<ResultContainer>();
            assert_eq!(results.get(), &["run".to_string(), "run".to_string()]);
        },
    );
}

#[test]
pub fn chain_call() {
    run_test(
        ExecutorKind::SingleThread,
        |world| {
            let component0 = world.init_component::<C0>();
            let entity0 = world.spawn((C0, C1, C2)).id();
            world.send_event(UpdateEvent::new(entity0, component0));

            DefaultUpdateGraphRegistery::add_system::<(C0,), (C1,), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run C0->C1");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );

            DefaultUpdateGraphRegistery::add_system::<(C1,), (C2,), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run C1->C2");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );
        },
        |world| {
            let mut results = world.resource_mut::<ResultContainer>();
            assert_eq!(
                results.get(),
                &["run C0->C1".to_string(), "run C1->C2".to_string()]
            );
        },
    );
}

#[test]
pub fn chain_call_multithread() {
    run_test(
        ExecutorKind::MultiThread,
        |world| {
            let component0 = world.init_component::<C0>();
            let entity0 = world.spawn((C0, C1, C2)).id();
            world.send_event(UpdateEvent::new(entity0, component0));

            DefaultUpdateGraphRegistery::add_system::<(C0,), (C1,), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run C0->C1");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );

            DefaultUpdateGraphRegistery::add_system::<(C1,), (C2,), _, _>(
                world,
                move |In(entitys): In<UpdateEntitys>, results: Res<ResultContainer>| {
                    results.push("run C1->C2");
                    assert_eq!(&*entitys.0, &[entity0]);
                },
            );
        },
        |world| {
            let mut results = world.resource_mut::<ResultContainer>();
            assert_eq!(
                results.get(),
                &["run C0->C1".to_string(), "run C1->C2".to_string()]
            );
        },
    );
}
