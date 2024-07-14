use bevy::{app::ScheduleRunnerPlugin, ecs::system::BoxedSystem, prelude::*, utils::HashSet};
use bevy_relationship_derive::graph_query2;

use bevy_relationship::{relationship, AppExt, ControlFlow, EntityCommandsExt};

#[derive(Component)]
pub struct C0(pub usize);

#[derive(Component)]
pub struct C1(pub usize);

#[derive(Component)]
pub struct C2(pub usize);

#[derive(Component)]
pub struct C3(pub usize);

relationship!(R0=>F0--T0);
relationship!(R1=>F1>-T1);
relationship!(R2=>F2-<T2);
relationship!(R3=>F3>-<T3);
relationship!(R4=>F4>-<T4);

fn test_suit(system: BoxedSystem) {
    let mut app = App::new();
    let systemid = app.world_mut().register_boxed_system(system);
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_once()))
        .register_relation::<R0>()
        .register_relation::<R1>()
        .register_relation::<R2>()
        .register_relation::<R3>()
        .register_relation::<R4>()
        .add_systems(Startup, move |mut command: Commands| {
            let e1 = command.spawn((C0(0), C3(0))).id();
            let e2 = command.spawn(C1(1)).id();
            let e3 = command.spawn(C2(2)).id();
            let e4 = command.spawn((C0(3), C1(3))).id();
            let e5 = command.spawn((C1(4), C2(4))).id();
            let e6 = command.spawn((C2(5), C0(5))).id();
            let e7 = command.spawn((C2(6), C0(6))).id();
            let e8 = command.spawn((C0(7), C1(7), C2(7))).id();
            assert_eq!(e1.index(), 1);
            assert_eq!(e2.index(), 2);
            assert_eq!(e3.index(), 3);
            assert_eq!(e4.index(), 4);
            assert_eq!(e5.index(), 5);
            assert_eq!(e6.index(), 6);
            assert_eq!(e7.index(), 7);
            assert_eq!(e8.index(), 8);
            command.entity(e1).connect_to::<R0>(e2);
            command.entity(e1).connect_to::<R1>(e3);
            command.entity(e1).connect_to::<R2>(e4);
            command.entity(e1).connect_to::<R2>(e4);
            command.entity(e1).connect_to::<R2>(e6);
            command.entity(e1).connect_to::<R2>(e8);
            command.entity(e1).connect_to::<R3>(e5);
            command.entity(e1).connect_to::<R3>(e8);
            command.entity(e2).connect_to::<R3>(e8);
            command.entity(e3).connect_to::<R3>(e8);
            command.entity(e4).connect_to::<R3>(e8);
            command.entity(e5).connect_to::<R3>(e8);
            command.entity(e6).connect_to::<R3>(e8);
            command.entity(e7).connect_to::<R3>(e8);
            command.entity(e1).connect_to::<R4>(e2);
            command.entity(e1).connect_to::<R4>(e3);
            command.entity(e2).connect_to::<R4>(e3);
            command.entity(e3).connect_to::<R4>(e4);
            command.entity(e3).connect_to::<R4>(e6);
            command.entity(e4).connect_to::<R4>(e5);
            command.entity(e5).connect_to::<R4>(e6);
            command.entity(e2).add_child(e3);
            command.entity(e2).add_child(e4);
            command.entity(e2).add_child(e5);
        })
        .add_systems(
            Update,
            (
                move |mut command: Commands| {
                    command.run_system(systemid);
                },
                apply_deferred,
            )
                .chain(),
        );
    app.run();
}

#[test]
fn test_query_node() {
    graph_query2! {
        QueryNode=> path=match ( n:&C0 )
    }

    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|e: &&C0| {
                ops.insert(e.0);
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([7, 3, 6, 5, 0]));
        },
    )));
}

#[test]
fn test_query_node_query_filter() {
    graph_query2! {
        QueryNode=> path=match ( n:Entity filter With<C0> )
    }

    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|e: &Entity| {
                ops.insert(e.index());
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([6, 4, 1, 8, 7]));
        },
    )));
}

#[test]
fn test_query_node_mut() {
    graph_query2! {
        QueryNode=>mut path=match ( n:&mut C0 )
    }

    test_suit(Box::new(IntoSystem::into_system(
        move |mut graph: QueryNode| {
            let mut ops = HashSet::new();
            let mut ops_mut = HashSet::new();
            let r = graph.foreach_path(|e: &&C0| {
                ops.insert(e.0);
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            let r = graph.foreach_path_mut(|mut e: &mut Mut<C0>| {
                let _: &mut C0 = &mut e;
                ops_mut.insert(e.0);
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([7, 3, 6, 5, 0]));
            assert_eq!(ops_mut, HashSet::from_iter([7, 3, 6, 5, 0]));
        },
    )));
}

#[test]
fn test_query_node_entity() {
    graph_query2! {
        QueryNode=> path=match ( n:( Entity,&C0 ) )
    }

    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, c): &(Entity, &C0)| {
                ops.insert((e.index(), c.0));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([(1, 0), (8, 7), (7, 6), (6, 5), (4, 3)])
            );
        },
    )));
}

#[test]
fn test_query_node_ref() {
    graph_query2! { QueryNode=> path=match ( n:Ref<C0> ) }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|c: &Ref<C0>| {
                ops.insert(c.0);
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([0, 7, 6, 3, 5]));
        },
    )));
}

#[test]
fn test_query_node_has() {
    graph_query2! {
        QueryNode=>mut path=match ( n:(Entity,Has<C0>) )
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, c): &(Entity, bool)| {
                ops.insert((e.index(), *c));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (0, false),
                    (1, true),
                    (2, false),
                    (3, false),
                    (4, true),
                    (5, false),
                    (6, true),
                    (7, true),
                    (8, true),
                ])
            );
        },
    )));
}

#[test]
fn test_query_node_option() {
    graph_query2! {
        QueryNode=>mut path=match ( n:(Entity,Option<&C0>) )
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, c): &(Entity, Option<&C0>)| {
                ops.insert((e.index(), c.is_some()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (0, false),
                    (1, true),
                    (2, false),
                    (3, false),
                    (4, true),
                    (5, false),
                    (6, true),
                    (7, true),
                    (8, true),
                ])
            );
        },
    )));
}

#[test]
fn test_query_node_tuple() {
    graph_query2! {
        QueryNode=>mut path=match ( n:(Entity,(&C0,&C1)) )
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, _c): &(Entity, (&C0, &C1))| {
                ops.insert(e.index());
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([8, 4]));
        },
    )));
}

#[test]
fn test_query_node_struct() {
    graph_query2! {
        QueryNode=>path=match ( node:(Entity,{ c0:&C0,c1:&C1 }) )
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, s)| {
                ops.insert((e.index(), s.c0.0, s.c1.0));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(4, 3, 3), (8, 7, 7)]));
        },
    )));
}

#[test]
fn test_query_node_where() {
    graph_query2! {
        QueryNode=>path=match ( node:(Entity,&C0) where node.1.0 < 6 )
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(e, c)| {
                ops.insert((e.index(), c.0));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(6, 5), (4, 3), (1, 0)]));
        },
    )));
}

#[test]
fn test_query_node_where_lambda() {
    graph_query2! {
        QueryNode=>path=match (node:(Entity,&C0) where ?)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(
                |(_, c)| c.0 < 6,
                |(e, c)| {
                    ops.insert((e.index(), c.0));
                    ControlFlow::<()>::Continue
                },
            );
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(6, 5), (4, 3), (1, 0)]));
        },
    )));
}

#[test]
fn test_query_path2() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[R3]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|n0, n1| {
                ops.insert((n0.index(), n1.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (2, 8),
                    (3, 8),
                    (7, 8),
                    (4, 8),
                    (1, 5),
                    (6, 8),
                    (5, 8),
                    (1, 8)
                ])
            );
        },
    )));
}

#[test]
fn test_query_path3() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[R3]->(node1:Entity)-[R3]->(node2:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|n0, n1, n2| {
                ops.insert((n0.index(), n1.index(), n2.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(1, 5, 8)]));
        },
    )));
}

#[test]
fn test_query_path_edge_filter() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[R3 where ?]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(
                |entity| entity.index() > 3,
                |n0, n1| {
                    ops.insert((n0.index(), n1.index()));
                    ControlFlow::<()>::Continue
                },
            );
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (1, 8),
                    (2, 8),
                    (6, 8),
                    (4, 8),
                    (5, 8),
                    (3, 8),
                    (1, 5),
                    (7, 8)
                ])
            );
        },
    )));
}

#[test]
fn test_query_path2_rev() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)<-[R3]-(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|n0, n1| {
                ops.insert((n0.index(), n1.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (8, 5),
                    (8, 2),
                    (8, 4),
                    (5, 1),
                    (8, 1),
                    (8, 7),
                    (8, 3),
                    (8, 6)
                ])
            );
        },
    )));
}

#[test]
fn test_query_children() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[HasChildren]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|n0, n1| {
                ops.insert((n0.index(), n1.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(2, 5), (2, 3), (2, 4)]));
        },
    )));
}

#[test]
fn test_query_edge_expr() {
    graph_query2! {
        QueryNode=>path=match (node0:(Entity,&Children))-[(node0.1.iter().cloned())]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|(n0, _children), n1| {
                ops.insert((n0.index(), n1.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(2, 5), (2, 3), (2, 4)]));
        },
    )));
}

#[test]
fn test_query_edge_lambda() {
    graph_query2! {
        QueryNode=>path=match (node0:(Entity,&Children))-[?]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(
                |(_n0, children)| children.iter().cloned().collect::<Vec<_>>(),
                |(n0, _children), n1| {
                    ops.insert((n0.index(), n1.index()));
                    ControlFlow::<()>::Continue
                },
            );
            assert!(r.is_none());
            assert_eq!(ops, HashSet::from_iter([(2, 5), (2, 3), (2, 4)]));
        },
    )));
}

#[test]
fn test_query_edge_many() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[[R2|R3]]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode| {
            let mut ops = HashSet::new();
            let r = graph.foreach_path(|n0, n1| {
                ops.insert((n0.index(), n1.index()));
                ControlFlow::<()>::Continue
            });
            assert!(r.is_none());
            assert_eq!(
                ops,
                HashSet::from_iter([
                    (1, 4),
                    (1, 5),
                    (1, 6),
                    (1, 8),
                    (2, 8),
                    (3, 8),
                    (4, 8),
                    (5, 8),
                    (6, 8),
                    (7, 8),
                ])
            );
        },
    )));
}

#[test]
fn test_query_from_node() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[R3]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode, query: Query<Entity, With<C0>>| {
            let mut ops = HashSet::new();
            for entity in &query {
                let r = graph.foreach_path_from(entity, |n0, n1| {
                    ops.insert((n0.index(), n1.index()));
                    ControlFlow::<()>::Continue
                });
                assert!(r.is_none());
            }
            assert_eq!(
                ops,
                HashSet::from_iter([(1, 5), (4, 8), (1, 8), (6, 8), (7, 8)])
            );
        },
    )));
}

#[test]
fn test_query_edge_deep() {
    graph_query2! {
        QueryNode=>path=match (node0:Entity)-[R4 * 1..2]->(node1:Entity)
    }
    test_suit(Box::new(IntoSystem::into_system(
        move |graph: QueryNode, query: Query<Entity, With<C3>>| {
            let mut ops = HashSet::new();
            for entity in &query {
                let r = graph.foreach_path_from(entity, |n0, n1| {
                    ops.insert((n0.index(), n1.index()));
                    ControlFlow::<()>::Continue
                });
                assert!(r.is_none());
            }
            assert_eq!(ops, HashSet::from_iter([(1, 6), (1, 4), (1, 3), (1, 2)]));
        },
    )));
}
