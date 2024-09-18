use std::{cell::UnsafeCell, collections::BTreeMap, marker::PhantomData, sync::Arc};

use bevy::{
    app::MainScheduleOrder,
    ecs::{
        archetype::{Archetype, ArchetypeGeneration, ArchetypeId},
        component::{ComponentId, Components},
        event::{EventId, ManualEventReader},
        schedule::ScheduleLabel,
        system::{BoxedSystem, SystemParam},
        world::unsafe_world_cell::UnsafeWorldCell,
    },
    prelude::*,
    tasks::ComputeTaskPool,
    utils::hashbrown::HashMap,
};
use futures::future::join_all;
use petgraph::{
    algo::toposort,
    graph::NodeIndex,
    visit::{depth_first_search, Control, ControlFlow, DfsEvent},
    Directed,
};
use smallvec::{smallvec, SmallVec};
use tokio::sync::{Notify, Semaphore};

#[derive(Clone, Event)]
pub struct UpdateEvent {
    entity: Entity,
    component: ComponentId,
}

impl UpdateEvent {
    pub fn new(entity: Entity, component: ComponentId) -> Self {
        Self { entity, component }
    }
}

#[derive(SystemParam)]
pub struct UpdateEventWriter<'w> {
    components: &'w Components,
    inner: EventWriter<'w, UpdateEvent>,
}

impl<'w> UpdateEventWriter<'w> {
    pub fn send<C: bevy::prelude::Component>(
        &mut self,
        entity: Entity,
    ) -> Option<EventId<UpdateEvent>> {
        self.components.component_id::<C>().map(|component_id| {
            self.inner.send(UpdateEvent {
                entity,
                component: component_id,
            })
        })
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub(crate) struct AccessFilters {
    pub(crate) read: Vec<ComponentId>,
    pub(crate) write: Vec<ComponentId>,
    pub(crate) subscribe: Vec<ComponentId>,
    pub(crate) publish: Vec<ComponentId>,
}

pub struct UpdateEntitys(pub SmallVec<[Entity; 8]>);

type Systems = Arc<SmallVec<[SystemIndex; 4]>>;
type Entitys = SmallVec<[Entity; 8]>;

type SortKey = u32;
type SystemIndex = u32;
type Task = (NodeIndex, SystemIndex, Entitys);

#[derive(Debug)]
struct SystemInfo {
    system_index: SystemIndex,
    access_filters: AccessFilters,
}

impl SystemInfo {
    pub fn accept_archtype(&self, archetype: &Archetype) -> bool {
        let check_component = |c: &ComponentId| archetype.contains(*c);
        self.access_filters.subscribe.iter().all(check_component)
            && self.access_filters.publish.iter().all(check_component)
    }
}

#[derive(Debug)]
enum Node {
    System(SystemInfo),
    Component,
}

#[derive(Default)]
pub struct MultiThreadScheduleCache {
    pub notifys: HashMap<ComponentId, (Semaphore, Notify, usize)>,
}

struct SystemCell {
    system: UnsafeCell<BoxedSystem<UpdateEntitys>>,
    node_index: NodeIndex,
    sort_key: SortKey,
}
unsafe impl Sync for SystemCell {
}

#[derive(Resource)]
pub struct UpdateGraphRegistry<Label: ScheduleLabel + Default = TriggerScheduleLabel> {
    systems: Vec<SystemCell>,
    groups: HashMap<(ArchetypeId, ComponentId), Systems>,
    archetype_generation: ArchetypeGeneration,

    component_nodes: HashMap<ComponentId, NodeIndex>,
    dependency_graph: petgraph::graph::Graph<Node, (), Directed, SystemIndex>,

    event_reader: ManualEventReader<UpdateEvent>,
    phantom: PhantomData<Label>,
}

impl<Label: ScheduleLabel + Default> FromWorld for UpdateGraphRegistry<Label> {
    fn from_world(world: &mut World) -> Self {
        Self {
            systems: Default::default(),
            groups: Default::default(),
            archetype_generation: ArchetypeGeneration::initial(),
            dependency_graph: Default::default(),
            component_nodes: Default::default(),
            phantom: PhantomData,
            event_reader: ManualEventReader::from_world(world),
        }
    }
}

impl<Label: ScheduleLabel + Default> UpdateGraphRegistry<Label> {
    fn add_raw_system(
        world: &mut World,
        mut system: BoxedSystem<UpdateEntitys>,
        input: &[ComponentId],
        output: &[ComponentId],
    ) {
        system.initialize(world);

        let mut this = world.resource_mut::<Self>();

        let access = system.component_access();
        let access_filters = AccessFilters {
            read: access.reads().collect(),
            write: access.writes().collect(),
            subscribe: input.to_vec(),
            publish: output.to_vec(),
        };
        let system_index = this.systems.len().try_into().unwrap();
        let systeminfo = SystemInfo {
            system_index,
            access_filters: access_filters.clone(),
        };

        let system_node = this.dependency_graph.add_node(Node::System(systeminfo));

        let system_cell = SystemCell {
            system: UnsafeCell::new(system),
            node_index: system_node,
            sort_key: system_index,
        };
        this.systems.push(system_cell);

        for id in input {
            let component_node = this.get_or_insert_component_node(*id);
            this.dependency_graph
                .add_edge(component_node, system_node, ());
        }

        for id in output {
            let component_node = this.get_or_insert_component_node(*id);
            this.dependency_graph
                .add_edge(system_node, component_node, ());
        }
    }

    pub fn add_system<I, O, M, S>(world: &mut World, system: S)
    where
        S: IntoSystem<UpdateEntitys, (), M>,
        I: Bundle,
        O: Bundle,
    {
        let mut system: S::System = IntoSystem::into_system(system);
        system.initialize(world);

        let input_bundle = world.init_bundle::<I>().components().to_vec();
        let output_bundle = world.init_bundle::<O>().components().to_vec();

        Self::add_raw_system(world, Box::new(system), &input_bundle, &output_bundle);
    }

    fn get_or_insert_component_node(&mut self, id: ComponentId) -> NodeIndex {
        *self.component_nodes.entry(id).or_insert_with(|| {
            (self.dependency_graph.add_node(Node::Component).index() as u32).into()
        })
    }

    pub fn build(&mut self) {
        let order = toposort(&self.dependency_graph, None).unwrap();
        for (index, node_index) in order.into_iter().enumerate() {
            if let Node::System(systeminfo) = &self.dependency_graph[node_index] {
                self.systems[systeminfo.system_index as usize].sort_key = index as u32;
            }
        }
    }

    pub fn update_archetypes(&mut self, world: &UnsafeWorldCell) {
        if world.archetypes().generation() == self.archetype_generation {
            return;
        }
        let old_generation = std::mem::replace(
            &mut self.archetype_generation,
            world.archetypes().generation(),
        );
        let mut new_triggers: HashMap<(ArchetypeId, ComponentId), SmallVec<[SystemIndex; 4]>> =
            HashMap::new();

        for archetype in &world.archetypes()[old_generation..] {
            for component_id in archetype.components() {
                if let Some(component_node_index) = self.component_nodes.get(&component_id) {
                    depth_first_search(&self.dependency_graph, [*component_node_index], |event| {
                        match event {
                            DfsEvent::Discover(node_index, _) => {
                                let node = &self.dependency_graph[node_index];
                                if let Node::System(systeminfo) = node {
                                    if systeminfo.accept_archtype(archetype) {
                                        new_triggers
                                            .entry((archetype.id(), component_id))
                                            .or_default()
                                            .push(systeminfo.system_index);
                                        return Control::continuing();
                                    }
                                    Control::Prune
                                } else {
                                    Control::continuing()
                                }
                            }
                            _ => Control::<()>::continuing(),
                        }
                    });
                }
            }
        }

        if new_triggers.len() > 0 {
            for ((archetype_id, component_id), systems) in new_triggers {
                self.groups
                    .insert((archetype_id, component_id), Arc::new(systems));
            }
        }
    }

    pub fn run_once(world: &mut World, entity: Entity, component: ComponentId) {
        let archetype_id = world.entity(entity).archetype().id();

        world.resource_scope(|world, mut this: Mut<Self>| {
            let Self {
                groups: trigger,
                systems,
                ..
            } = &mut *this;
            let Some(system_indexs) = trigger.get(&(archetype_id, component)) else {
                return;
            };
            for system_index in &system_indexs[..] {
                systems[*system_index as usize]
                    .system
                    .get_mut()
                    .run(UpdateEntitys(smallvec![entity]), world);
            }
        });
    }

    fn schedule(&mut self, world: &mut World) -> BTreeMap<SortKey, Task> {
        let mut tasks: BTreeMap<SortKey, Task> = BTreeMap::new();

        for &UpdateEvent {
            entity,
            component: component_id,
        } in self.event_reader.read(world.get_resource().unwrap())
        {
            let Some(entity_ref) = world.get_entity(entity) else {
                continue;
            };
            let archetype_id = entity_ref.archetype().id();
            if let Some(systems) = self.groups.get(&(archetype_id, component_id)) {
                for &system_index in &**systems {
                    let sort_key = self.systems[system_index as usize].sort_key;
                    tasks
                        .entry(sort_key)
                        .or_insert_with(|| {
                            (
                                self.systems[system_index as usize].node_index,
                                system_index,
                                Default::default(),
                            )
                        })
                        .2
                        .push(entity);
                }
            }
        }
        tasks
    }

    pub fn run_single_thread(world: &mut World) {
        world.resource_scope(|world, mut this: Mut<Self>| {
            this.update_archetypes(&world.as_unsafe_world_cell());
            loop {
                let tasks = this.schedule(world);
                if tasks.is_empty() {
                    break;
                }

                for (_, (_, system_index, entitys)) in tasks {
                    this.systems[system_index as usize]
                        .system
                        .get_mut()
                        .run(UpdateEntitys(entitys), world);
                }
            }
        });
    }

    pub fn run_multi_thread(world: &mut World, mut cache: Local<MultiThreadScheduleCache>) {
        world.resource_scope(|world, mut this: Mut<Self>| loop {
            this.update_archetypes(&world.as_unsafe_world_cell());
            let tasks = this.schedule(world);
            if tasks.is_empty() {
                break;
            }

            cache.notifys.clear();

            for (_, system_index, _) in tasks.values() {
                let node_index = this.systems[*system_index as usize].node_index;
                if let Node::System(systeminfo) = &this.dependency_graph[node_index] {
                    for component_id in &systeminfo.access_filters.write {
                        cache
                            .notifys
                            .entry(*component_id)
                            .or_insert_with(|| (Semaphore::new(0), Notify::new(), 0))
                            .2 += 1;
                    }
                } else {
                    unreachable!()
                };
            }

            let notifys = &cache.notifys;
            let systems = &this.systems;
            let dependency_graph = &this.dependency_graph;
            let world_cell = world.as_unsafe_world_cell();

            ComputeTaskPool::get().scope(|scope| {
                for (sem, notify, count) in notifys.values() {
                    if *count != 1 {
                        scope.spawn(async {
                            let _ = sem.acquire().await.unwrap();
                            notify.notify_waiters();
                        });
                    }
                }
                for (_, (node_index, system_index, entitys)) in tasks {
                    if let Node::System(systeminfo) = &dependency_graph[node_index] {
                        scope.spawn(async move {
                            join_all(systeminfo.access_filters.read.iter().filter_map(
                                |component_id| notifys.get(component_id).map(|n| n.1.notified()),
                            ))
                            .await;

                            unsafe {
                                systems[system_index as usize]
                                    .system
                                    .get()
                                    .as_mut()
                                    .unwrap()
                                    .run_unsafe(UpdateEntitys(entitys), world_cell);
                            }

                            for component_id in systeminfo.access_filters.write.iter() {
                                let (sem, notify, count) = &notifys[component_id];
                                if *count == 1 {
                                    notify.notify_waiters();
                                } else {
                                    sem.add_permits(0);
                                }
                            }
                        });
                    } else {
                        unreachable!()
                    };
                }
            });
        });
    }
}

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Clone, Debug, Default)]
pub struct TriggerScheduleLabel;

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutorKind {
    SingleThread,
    #[default]
    MultiThread,
}

#[derive(Default)]
pub struct TriggerPlugin<Label: ScheduleLabel + Default = TriggerScheduleLabel> {
    pub executor_kind: ExecutorKind,
    pub phantom: PhantomData<Label>,
}

impl<Label: ScheduleLabel + Default> Plugin for TriggerPlugin<Label> {
    fn build(&self, app: &mut App) {
        app.add_event::<UpdateEvent>();
        app.init_resource::<UpdateGraphRegistry<Label>>();
        let mut schedule = Schedule::new(Label::default());

        match &self.executor_kind {
            ExecutorKind::SingleThread => {
                schedule.add_systems(
                    UpdateGraphRegistry::<Label>::run_single_thread
                        .run_if(on_event::<UpdateEvent>()),
                );
            }
            ExecutorKind::MultiThread => {
                schedule.add_systems(
                    UpdateGraphRegistry::<Label>::run_multi_thread
                        .run_if(on_event::<UpdateEvent>()),
                );
            }
        }
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Update, Label::default());
        app.add_schedule(schedule);
    }

    fn finish(&self, app: &mut App) {
        let mut this = app.world_mut().resource_mut::<UpdateGraphRegistry<Label>>();
        this.build();
    }
}

pub type DefaultUpdateGraphRegistery = UpdateGraphRegistry<TriggerScheduleLabel>;

#[cfg(test)]
mod tests {
    extern crate test;
    use std::sync::Mutex;

    use bevy::prelude::*;
    use test::Bencher;

    use super::*;

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

    pub fn run_bench(b: &mut Bencher, init: impl FnOnce(&mut World) -> Vec<UpdateEvent>) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(TriggerPlugin::<TriggerScheduleLabel>::default());
        app.init_resource::<ResultContainer>();
        let events = init(app.world_mut());
        app.finish();

        b.iter(|| {
            app.world_mut().send_event_batch(events.iter().cloned());
            DefaultUpdateGraphRegistery::run_single_thread(app.world_mut());
        });
    }

    #[test]
    pub fn sinsgle_system() {
        run_test(
            ExecutorKind::SingleThread,
            |world| {
                let component0 = world.init_component::<C0>();
                let entity0 = world.spawn(C0).id();
                world.send_event(UpdateEvent {
                    entity: entity0,
                    component: component0,
                });

                DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
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
                world.send_event(UpdateEvent {
                    entity: entity0,
                    component: component0,
                });

                DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
                        results.push("run");
                        assert_eq!(&*entitys.0, &[entity0]);
                    },
                );

                DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
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
                world.send_event(UpdateEvent {
                    entity: entity0,
                    component: component0,
                });

                DefaultUpdateGraphRegistery::add_system::<(C0,), (C1,), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
                        results.push("run C0->C1");
                        assert_eq!(&*entitys.0, &[entity0]);
                    },
                );

                DefaultUpdateGraphRegistery::add_system::<(C1,), (C2,), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
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
                world.send_event(UpdateEvent {
                    entity: entity0,
                    component: component0,
                });

                DefaultUpdateGraphRegistery::add_system::<(C0,), (C1,), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
                        results.push("run C0->C1");
                        assert_eq!(&*entitys.0, &[entity0]);
                    },
                );

                DefaultUpdateGraphRegistery::add_system::<(C1,), (C2,), _, _>(
                    world,
                    move |In(entitys): In<super::UpdateEntitys>, results: Res<ResultContainer>| {
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

    #[bench]
    pub fn bench_single_thread_0_events(b: &mut Bencher) {
        run_bench(b, |world| {
            DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                world,
                move |In(_entitys): In<super::UpdateEntitys>| {},
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
                move |In(_entitys): In<super::UpdateEntitys>| {},
            );

            (0..1)
                .map(|_| UpdateEvent {
                    entity: world.spawn(C0).id(),
                    component: component0,
                })
                .collect()
        });
    }

    #[bench]
    pub fn bench_single_thread_1000_events(b: &mut Bencher) {
        run_bench(b, |world| {
            let component0 = world.init_component::<C0>();

            DefaultUpdateGraphRegistery::add_system::<(C0,), (), _, _>(
                world,
                move |In(entitys): In<super::UpdateEntitys>| {
                    assert_eq!(entitys.0.len(), 1000);
                },
            );

            (0..1000)
                .map(|_| UpdateEvent {
                    entity: world.spawn(C0).id(),
                    component: component0,
                })
                .collect()
        });
    }
}
