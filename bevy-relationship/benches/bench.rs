#![feature(array_windows)]
use std::collections::VecDeque;

use bevy::{
    app::ScheduleRunnerPlugin,
    ecs::{
        entity,
        system::{Command, RunSystemOnce, SystemParam},
    },
    prelude::*,
};
use bevy_relationship::*;
use bevy_relationship_derive::graph_query2;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::seq::SliceRandom;

#[derive(Component)]
pub struct C0(pub usize);

#[derive(Component)]
pub struct C1(pub usize);

#[derive(Component)]
pub struct C2(pub usize);

#[derive(Component)]
pub struct C3(pub usize);

#[derive(Component)]
pub struct Root;

relationship!(WzGraph=> @both -<WzPeer);
relationship!(Tree=>F1>-<T1);
relationship!(RandomGraph=>F2>-<T2);
relationship!(R3=>F3>-<T3);
relationship!(R4=>F4>-<T4);

fn init_wz_graph(world: &mut World, entitys: &[Entity], k: usize, beta: f32) {
    let mut rng = rand::thread_rng();

    ConnectCommand::<WzGraph>::new(*entitys.last().unwrap(), entitys[0]).apply(world);
    for window in entitys.windows(k + 1) {
        let node = window[0];
        for peer in &window[1..] {
            ConnectCommand::<WzGraph>::new(node, *peer).apply(world);
        }
    }

    for i in 0..entitys.len() {
        if rand::random::<f32>() > beta {
            continue;
        }
        let node = entitys[i];
        DisconnectAllCommand::<WzGraph>::new(node).apply(world);
        for peer in entitys.choose_multiple(&mut rng, k) {
            ConnectCommand::<WzGraph>::new(node, *peer).apply(world);
        }
    }
}

fn init_tree(world: &mut World, entitys: &[Entity], w: usize) {
    world.entity_mut(entitys[0]).insert(Root);
    let mut iter = entitys[1..].chunks(w);
    let mut queue = VecDeque::new();
    queue.push_back(entitys[0]);
    while let Some(root) = queue.pop_front() {
        if let Some(chunk) = iter.next(){
            for peer in chunk {
                ConnectCommand::<Tree>::new(root, *peer).apply(world);
            }
            queue.extend(chunk);
        }
    }
}

fn init_random_graph(world: &mut World, entitys: &[Entity], k: usize) {
    let mut rng = rand::thread_rng();
    for e in entitys {
        for p in entitys.choose_multiple(&mut rng, k) {
            ConnectCommand::<RandomGraph>::new(*e, *p).apply(world);
        }
    }
}

fn create_random_graph() -> (App, Vec<Entity>) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .register_relation::<WzGraph>()
        .register_relation::<Tree>()
        .register_relation::<RandomGraph>()
        .register_relation::<R3>()
        .register_relation::<R4>();
    let world = &mut app.world;
    let entitys = (0..1024)
        .map(|i| world.spawn(C0(i)).id())
        .collect::<Vec<_>>();
    init_wz_graph(world, &entitys, 1, 0.0);
    init_tree(world, &entitys, 32);
    init_random_graph(world, &entitys, 1);
    app.finish();
    app.cleanup();
    (app, entitys)
}

fn bench_iter(c: &mut Criterion) {
    let (mut app, mut entitys) = create_random_graph();
    let mut rng = rand::thread_rng();
    entitys.shuffle(&mut rng);
    let world = &mut app.world;

    c.bench_function("iter_entity", |b| {
        let mut query = QueryState::<Entity, With<C0>>::new(world);
        b.iter(|| {
            let mut r: u64 = 0;
            for entity in query.iter(world) {
                r += entity.to_bits();
            }
            black_box(r);
        });
    });

    c.bench_function("get_entity_random", |b| {
        let mut query = QueryState::<Entity, With<C0>>::new(world);
        b.iter(|| {
            let mut r: u64 = 0;
            for entity in &entitys {
                if let Ok(entity) = query.get(world, *entity) {
                    r += entity.to_bits();
                }
            }
            black_box(r);
        });
    });

    c.bench_function("iter_many_entity_random", |b| {
        let mut query = QueryState::<Entity, With<C0>>::new(world);
        b.iter(|| {
            let mut r: u64 = 0;
            for entity in query.iter_many(world, entitys.clone()) {
                r += entity.to_bits();
            }
            black_box(r);
        });
    });

    c.bench_function("query_path2_manual", |b| {
        b.iter(|| {
            app.world
                .run_system_once(|query: Query<(Entity, &WzPeer)>, query2: Query<Entity>| {
                    let mut r: u64 = 0;
                    for (n0, f0) in query.iter() {
                        let mut iterator = query2.iter_many(f0.iter());
                        while let Some(n1) = iterator.fetch_next() {
                            r += n0.to_bits() + n1.to_bits();
                        }
                    }
                    black_box(r);
                })
        });
    });

    c.bench_function("query_path2", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)-[WzGraph]->(node1:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                graph.foreach_path(|n0, n1| {
                    r += n0.to_bits() + n1.to_bits();
                    ControlFlow::<()>::Continue
                });
                black_box(r);
            })
        });
    });

    c.bench_function("query_path3", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)-[WzGraph]->(node1:Entity)-[WzGraph]->(node2:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                graph.foreach_path(|n0, n1, n2| {
                    r += n0.to_bits() + n1.to_bits() + n2.to_bits();
                    ControlFlow::<()>::Continue
                });
                black_box(r);
            })
        });
    });

    c.bench_function("query_tree_path2", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)-[Tree]->(node1:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                let mut c = 0;
                graph.foreach_path(|n0, n1| {
                    r += n0.to_bits() + n1.to_bits();
                    c += 1;
                    ControlFlow::<()>::Continue
                });
                assert_eq!(c, 1023);
                black_box(r);
            })
        });
    });

    c.bench_function("query_tree_path3", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity filter With<Root>)-[Tree]->(node1:Entity)-[Tree]->(node2:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                let mut c =0;
                graph.foreach_path(|n0, n1, n2| {
                    r += n0.to_bits() + n1.to_bits() + n2.to_bits();
                    c+=1;
                    ControlFlow::<()>::Continue
                });
                assert_eq!(c,991);
                black_box(r);
            })
        });
    });

    c.bench_function("query_random_graph_path1", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                graph.foreach_path(|n0| {
                    r += n0.to_bits();
                    ControlFlow::<()>::Continue
                });
                black_box(r);
            })
        });
    });

    c.bench_function("query_random_graph_path2", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)-[RandomGraph]->(node1:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                graph.foreach_path(|n0, n1| {
                    r += n0.to_bits() + n1.to_bits();
                    ControlFlow::<()>::Continue
                });
                black_box(r);
            })
        });
    });

    c.bench_function("query_random_graph_path3", |b| {
        b.iter(|| {
            graph_query2! {
                QueryGraph=>path=match (node0:Entity)-[RandomGraph]->(node1:Entity)-[RandomGraph]->(node2:Entity)
            }

            app.world.run_system_once(|graph: QueryGraph| {
                let mut r: u64 = 0;
                graph.foreach_path(|n0, n1, n2| {
                    r += n0.to_bits() + n1.to_bits() + n2.to_bits();
                    ControlFlow::<()>::Continue
                });
                black_box(r);
            })
        });
    });
}

criterion_group!(benches, bench_iter);
criterion_main!(benches);
