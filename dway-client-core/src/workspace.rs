use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::{toplevel::DWayToplevel, DWayWindow},
};
use dway_util::update;

use crate::{
    prelude::*,
    screen::{create_screen, Screen},
    window::Hidden,
};

#[derive(Component, Default, Debug, Reflect)]
pub struct WorkspaceSet;

#[derive(Component, Default, Debug, Reflect)]
pub struct WorkspaceWindow {
    pub hide: bool,
}

#[derive(Resource, Default, Debug, Reflect)]
pub struct WorkspaceManager {
    pub workspaces: Vec<Entity>,
}

structstruck::strike! {
    pub struct WorkspaceRequest {
        pub workspace: Entity,
        pub kind: pub enum WorkspaceRequestKind {
            AttachToScreen{
                screen: Entity,
                unique: bool,
            },
            LeaveScreen{
                screen: Entity,
            },
            AttachWindow{
                window: Entity,
            },
            RemoveWindow{
                window: Entity,
            }
        }
    }
}

#[derive(Component, Default, Debug, Reflect)]
pub struct Workspace {
    pub name: String,
    pub hide: bool,
}

#[derive(Bundle, Default)]
pub struct WorkspaceBundle {
    pub workspace: Workspace,
    pub geo: Geometry,
    pub global: GlobalGeometry,

    pub window_list: WindowList,
    pub screen_list: ScreenList,
}

relationship!(WindowOnWorkspace=>WindowWorkspaceList>-<WindowList);
relationship!(ScreenAttachWorkspace=>ScreenWorkspaceList>-<ScreenList);

pub fn attach_window_to_workspace(
    new_window: Query<(Entity, &GlobalGeometry)>,
    mut insert_window_event: EventReader<Insert<DWayWindow>>,
    workspace_query: Query<(Entity, &Workspace, &GlobalGeometry)>,
    mut commands: Commands,
) {
    for (window, window_geo) in new_window.iter_many(insert_window_event.read().map(|e| e.entity)) {
        for (workspace_entity, workspace, workspace_geo) in workspace_query.iter() {
            if !workspace.hide
                && workspace_geo.intersection(window_geo.geometry).size() != IVec2::default()
            {
                commands.entity(window).insert(WorkspaceWindow {
                    hide: workspace.hide,
                });
                commands.add(ConnectCommand::<WindowOnWorkspace>::new(
                    window,
                    workspace_entity,
                ));
            }
        }
    }
}

pub fn attach_workspace_to_screen(
    screen_query: Query<(Entity, &GlobalGeometry), Added<Screen>>,
    mut workspace_query: Query<
        (Entity, &mut Workspace, &mut Geometry),
        (With<Workspace>, Without<Hidden>),
    >,
    mut new_screen: EventReader<Insert<Screen>>,
    mut commands: Commands,
) {
    for (screen_entity, screen_geo) in screen_query.iter_many(new_screen.read().map(|e| e.entity)) {
        for (workspace_entity, workspace, mut workspace_geo) in workspace_query.iter_mut() {
            if !workspace.hide {
                commands.add(ConnectCommand::<ScreenAttachWorkspace>::new(
                    screen_entity,
                    workspace_entity,
                ));
                workspace_geo.geometry = screen_geo.geometry;
            }
        }
    }
}

graph_query2! {
WorkspaceGraph=>
mut workspaces=match
    (workspace:(Entity,&mut Workspace,Option<&ScreenList>) filter Or<(Changed<Workspace>,Changed<ScreenList>)>);
}

pub fn update_workspace_system(
    mut graph: WorkspaceGraph,
    mut workspace_manager: ResMut<WorkspaceManager>,
) {
    graph.foreach_workspaces_mut(|(entity, workspace, screen_list)| {
        if workspace.is_added() {
            workspace_manager.workspaces.push(*entity);
        }
        let hide = screen_list.map(|l| l.is_empty()).unwrap_or(true);
        update!(workspace.hide, hide);
        ControlFlow::<()>::Continue
    });
}

graph_query2! {
WorkspaceWindowGraph=>
mut workspaces_to_window=match
    (workspace:&Workspace filter Or<(Changed<WindowList>,Changed<Workspace>)>)
        <-[WindowOnWorkspace]-(window:&mut WorkspaceWindow);
}

pub fn update_workspace_window_system(mut graph: WorkspaceWindowGraph) {
    graph.foreach_workspaces_to_window_mut(|workspace, workspace_info| {
        update!(workspace_info.hide, workspace.hide);
        ControlFlow::<()>::Continue
    });
}

pub struct WorkspacePlugin;
impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowOnWorkspace>();
        app.register_relation::<ScreenAttachWorkspace>();
        app.register_type::<Workspace>();
        app.register_type::<WorkspaceManager>();
        app.register_type::<WorkspaceWindow>();
        app.init_resource::<WorkspaceManager>();
        app.add_systems(
            PreUpdate,
            (attach_workspace_to_screen, apply_deferred)
                .run_if(on_event::<Insert<Screen>>())
                .after(DWayClientSystem::CreateScreen)
                .before(DWayClientSystem::UpdateWorkspace),
        );
        app.add_systems(
            PreUpdate,
            (
                update_workspace_system.in_set(DWayClientSystem::UpdateWorkspace),
                (
                    (attach_window_to_workspace, apply_deferred)
                        .run_if(on_event::<Insert<DWayWindow>>()),
                    update_workspace_window_system,
                )
                    .chain()
                    .in_set(DWayClientSystem::UpdateWorkspace),
            )
                .in_set(DWayClientSystem::UpdateWorkspace),
        );
    }
}
