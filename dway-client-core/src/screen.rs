use dway_server::{
    events::Insert,
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::DWayWindow,
};

use crate::{
    prelude::*,
    window::WindowStatistics,
    workspace::{ScreenAttachWorkspace, Workspace, WorkspaceWindow},
};

#[derive(Component)]
pub struct Screen {
    pub name: String,
}

#[derive(Bundle)]
pub struct ScreenBundle {
    pub name: Name,
    pub geometry: Geometry,
    pub global_geometry: GlobalGeometry,
    pub screen: Screen,
    pub stat: WindowStatistics,
}

pub fn create_screen(
    screen_query: Query<(Entity, Ref<Window>, Option<&Screen>), Changed<Window>>,
    mut commands: Commands,
    mut event: MessageWriter<Insert<Screen>>,
) {
    for (entity, window, screen) in screen_query.iter() {
        let WindowPosition::At(_window_position) = window.position else {
            continue;
        };
        let rect = IRect::new(
            0,
            0,
            window.resolution.width() as i32,
            window.resolution.height() as i32,
        );
        if screen.is_none() {
            commands.entity(entity).insert(ScreenBundle {
                screen: Screen {
                    name: window.title.clone(),
                },
                name: Name::new(window.title.clone()),
                geometry: Geometry::new(rect),
                global_geometry: GlobalGeometry::new(rect),
                stat: WindowStatistics::default(),
            });
            event.write(Insert::new(entity));
        }
    }
}

structstruck::strike! {
    #[derive(EntityEvent)]
    pub struct ScreenNotify{
        #[event_target]
        pub screen: Entity,
        pub kind: pub enum ScreenNotifyKind {
            WindowEnter(Entity),
            WindowLeave(Entity),
        }
    }
}

impl ScreenNotify {
    pub fn new(screen: Entity, kind: ScreenNotifyKind) -> Self {
        Self { screen, kind }
    }
}

relationship!(ScreenContainsWindow=>ScreenWindowList>-<WindowScreenList);

pub fn update_screen(
    screen_query: Query<(Entity, Ref<GlobalGeometry>), With<Screen>>,
    window_query: Query<(
        Entity,
        Ref<GlobalGeometry>,
        Ref<DWayWindow>,
        Option<Ref<WorkspaceWindow>>,
    )>,
    mut commands: Commands,
) {
    let update = |(screen_entity, screen_geo): &(Entity, Ref<GlobalGeometry>),
                  (window_entity, window_geo, _, workspace_window): &(
        Entity,
        Ref<GlobalGeometry>,
        Ref<DWayWindow>,
        Option<Ref<WorkspaceWindow>>,
    ),
                  commands: &mut Commands| {
        if !screen_geo.intersection(window_geo.geometry).empty()
            && !workspace_window.as_ref().map(|w| w.hide).unwrap_or(false)
        {
            commands
                .entity(*screen_entity)
                .connect_to::<ScreenContainsWindow>(*window_entity);
            commands.trigger(ScreenNotify::new(
                *screen_entity,
                ScreenNotifyKind::WindowEnter(*window_entity),
            ));
        }
    };
    for w in &window_query {
        let (window_entity, window_geo, ref window, workspace_window) = &w;
        if window_geo.is_changed()
            || workspace_window
                .as_ref()
                .map(|x| x.is_changed())
                .unwrap_or(false)
            || window.is_changed()
        {
            commands
                .entity(*window_entity)
                .disconnect_all_rev::<ScreenContainsWindow>();
            for s in &screen_query {
                update(&s, &w, &mut commands);
            }
        }
    }
    for ref s @ (screen_entity, ref screen_geo) in &screen_query {
        if screen_geo.is_changed() {
            commands
                .entity(screen_entity)
                .disconnect_all::<ScreenContainsWindow>();
            for w in &window_query {
                update(s, &w, &mut commands);
            }
        }
    }
}

graph_query2! {
ScreenGraph=>
    window_to_screen=match (window:(&DWayWindow,&GlobalGeometry)) -[WindowOnWorkspace]->(w:&Workspace) <-[ScreenAttachWorkspace]-(s:(&Screen,&GlobalGeometry));
}

pub fn update_screen_system() {
}

pub struct ScreenPlugin;
impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<ScreenContainsWindow>();
        app.add_event::<Insert<Screen>>();
        app.add_systems(
            PreUpdate,
            (
                create_screen.in_set(DWayClientSystem::CreateScreen),
                update_screen.in_set(DWayClientSystem::UpdateScreen),
            )
                .chain(),
        );
    }
}
