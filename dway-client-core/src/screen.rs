use crate::{
    prelude::*,
    workspace::{ScreenAttachWorkspace, Workspace, WorkspaceWindow},
};
use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::DWayWindow,
};

#[derive(Component)]
pub struct Screen {
    pub name: String,
}

pub fn create_screen(
    screen_query: Query<(Entity, Ref<Window>, Option<&Screen>), Changed<Window>>,
    mut commands: Commands,
    mut event: EventWriter<Insert<Screen>>,
) {
    for (entity, window, screen) in screen_query.iter() {
        let WindowPosition::At(window_position) = window.position else {
            continue;
        };
        let rect = IRect::new(
            window_position.x,
            window_position.y,
            window.resolution.width() as i32,
            window.resolution.height() as i32,
        );
        if screen.is_none() {
            commands.entity(entity).insert((
                Screen {
                    name: window.title.clone(),
                },
                Name::new(window.title.clone()),
                Geometry::new(rect),
                GlobalGeometry::new(rect),
            ));
            event.send(Insert::new(entity));
        }
    }
}

relationship!(ScreenContainsWindow=>ScreenWindowList>-<WindowScreenList);

pub fn update_screen(
    screen_query: Query<(Entity, Ref<GlobalGeometry>)>,
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
        }
    };
    for w in &window_query {
        let (window_entity, window_geo, ref window, workspace_window) = &w;
        if window_geo.is_changed()
            || workspace_window.as_ref().map(|x| x.is_changed()).unwrap_or(false)
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
                update(&s, &w, &mut commands);
            }
        }
    }
}

graph_query2! {
ScreenGraph=>
    window_to_screen=match (window:(&DWayWindow,&GlobalGeometry)) -[WindowOnWorkspace]->(w:(&Workspace)) <-[ScreenAttachWorkspace]-(s:(&Screen,&GlobalGeometry));
}

pub fn update_screen_system() {}

pub struct ScreenPlugin;
impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<ScreenContainsWindow>();
        app.add_event::<Insert<Screen>>();
        app.add_systems(
            PreUpdate,
            (
                create_screen.in_set(DWayClientSystem::CreateComponent),
                update_screen.in_set(DWayClientSystem::CreateComponent),
            )
                .chain(),
        );
    }
}
