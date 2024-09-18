#![feature(test)]
extern crate test;

use bevy::prelude::*;
use bevy_relationship::{
    DefaultUpdateGraphRegistery, TriggerPlugin, TriggerScheduleLabel, UpdateEntitys, UpdateEvent,
};
use test::Bencher;

#[derive(Component, Clone, Copy)]
pub struct C0;

pub fn run_bench(b: &mut Bencher, init: impl FnOnce(&mut World) -> Vec<UpdateEvent>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(TriggerPlugin::<TriggerScheduleLabel>::default());
    let events = init(app.world_mut());
    app.finish();

    b.iter(|| {
        app.world_mut().send_event_batch(events.iter().cloned());
        DefaultUpdateGraphRegistery::run_single_thread(app.world_mut());
    });
}

#[bench]
pub fn bench_single_thread_0_events(b: &mut Bencher) {
    run_bench(b, |world| {
        DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
            world,
            move |In(_entitys): In<UpdateEntitys>| {},
        );
        vec![]
    });
}

#[bench]
pub fn bench_single_thread_1_events(b: &mut Bencher) {
    run_bench(b, |world| {
        let component0 = world.init_component::<C0>();

        DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
            world,
            move |In(_entitys): In<UpdateEntitys>| {},
        );

        (0..1)
            .map(|_| UpdateEvent::new(world.spawn(C0).id(), component0))
            .collect()
    });
}

#[bench]
pub fn bench_single_thread_1000_events(b: &mut Bencher) {
    run_bench(b, |world| {
        let component0 = world.init_component::<C0>();

        DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
            world,
            move |In(entitys): In<UpdateEntitys>| {
                assert_eq!(entitys.0.len(), 1000);
            },
        );

        (0..1000)
            .map(|_| UpdateEvent::new(world.spawn(C0).id(), component0))
            .collect()
    });
}
