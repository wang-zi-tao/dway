use dway_server::geometry::{Geometry, GlobalGeometry};
use dway_util::update;
use smart_default::SmartDefault;

use crate::{
    desktop::{CursorOnScreen, FocusedWindow},
    prelude::*,
    screen::{
        Screen, ScreenNotify,
    },
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

pub fn on_new_workspace(
    trigger: Trigger<OnInsert, Workspace>,
    mut workspace_manager: ResMut<WorkspaceManager>,
) {
    workspace_manager.workspaces.push(trigger.target());
}
pub fn on_destroy_workspace(
    trigger: Trigger<OnRemove, Workspace>,
    mut workspace_manager: ResMut<WorkspaceManager>,
) {
    workspace_manager
        .workspaces
        .retain(|e| *e != trigger.target());
}

graph_query2! { FocusedWindowGraph=>
mut window_path=match (window:Entity)-[WindowOnWorkspace]->(workspace:( Entity,&Workspace ));
}

pub fn on_focus_window(
    window_focus: Res<FocusedWindow>,
    screen: Res<CursorOnScreen>,
    graph: FocusedWindowGraph,
    mut commands: Commands,
) {
    if !window_focus.is_changed() {
        return;
    }
    let Some(window) = window_focus.window_entity else {
        return;
    };
    let Some((screen, _)) = screen.0 else {
        return;
    };

    let attached = graph.foreach_window_path_from(window, |w, workspace| {
        if !workspace.1.no_screen {
            return ControlFlow::Return(());
        }
        ControlFlow::Continue
    });
    if attached.is_some() {
        return;
    }

    let has_show = graph.foreach_window_path_from(window, |w, workspace| {
        commands
            .entity(screen)
            .disconnect_all::<ScreenAttachWorkspace>()
            .connect_to::<ScreenAttachWorkspace>(workspace.0);
        ControlFlow::Return(())
    });
}

#[derive(Event)]
pub enum WorkspaceRequest {
    AttachToScreen { screen: Entity, unique: bool },
    LeaveScreen { screen: Entity },
    AttachWindow { window: Entity, unique: bool },
    RemoveWindow { window: Entity },
    UpdateWorkspace,
}

pub fn resolve_workspace_request(
    trigger: Trigger<WorkspaceRequest>,
    workspace_query: Query<&Workspace>,
    screen_query: Query<&Screen>,
    mut geometry_query: Query<&mut Geometry>,
    mut commands: Commands,
) {
    match trigger.event() {
        WorkspaceRequest::AttachToScreen { screen, unique } => {
            if *unique {
                commands
                    .entity(*screen)
                    .disconnect_all::<ScreenAttachWorkspace>();
            }
            commands
                .entity(trigger.target())
                .connect_from::<ScreenAttachWorkspace>(*screen);
            if let Ok([screen_geo, mut workspace_geo]) =
                geometry_query.get_many_mut([*screen, trigger.target()])
            {
                *workspace_geo = screen_geo.clone();
            };
        }
        WorkspaceRequest::LeaveScreen { screen } => {
            commands
                .entity(trigger.target())
                .disconnect_from::<ScreenAttachWorkspace>(*screen);
        }
        WorkspaceRequest::AttachWindow { window, unique } => {
            if *unique {
                commands
                    .entity(*window)
                    .disconnect_all::<WindowOnWorkspace>();
            }
            commands
                .entity(*window)
                .connect_to::<WindowOnWorkspace>(trigger.target());
        }
        WorkspaceRequest::RemoveWindow { window } => {
            commands
                .entity(*window)
                .disconnect_to::<WindowOnWorkspace>(trigger.target());
        }
        WorkspaceRequest::UpdateWorkspace => {}
    }
}

#[derive(Component, SmartDefault, Debug, Reflect)]
pub struct Workspace {
    pub name: String,
    pub hide: bool,
    #[default(true)]
    pub no_screen: bool,
}

impl Workspace {
    pub fn visiable(self) -> bool {
        !self.no_screen && !self.hide
    }
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
    trigger: Trigger<ScreenNotify>,
    window_query: Query<&WindowWorkspaceList>,
    screen_query: Query<&ScreenWorkspaceList>,
    geo_query: Query<&GlobalGeometry>,
    mut commands: Commands,
) {
    let ScreenNotify::WindowEnter(window_entity) = trigger.event() else {
        return;
    };
    let Ok(window_rect) = geo_query.get(*window_entity) else {
        warn!(entity=?window_entity, "the window has no GlobalGeometry");
        return;
    };

    if window_query
        .get(*window_entity)
        .map(|l| l.is_empty())
        .unwrap_or(true)
    {
        if let Ok(workspace_list) = screen_query.get(trigger.target()) {
            for workspace in workspace_list.iter() {
                let Ok(workspace_rect) = geo_query.get(workspace) else {
                    warn!(entity=?workspace, "the workspace has no GlobalGeometry");
                    continue;
                };
                if workspace_rect.intersection(window_rect.geometry).area() > 0 {
                    commands
                        .entity(*window_entity)
                        .insert(WorkspaceWindow::default())
                        .disconnect_all::<WindowOnWorkspace>()
                        .connect_to::<WindowOnWorkspace>(workspace);
                    return;
                }
            }
        }
    }
}

pub fn on_add_screen(
    trigger: Trigger<OnInsert, Screen>,
    screen_query: Query<(Entity, &GlobalGeometry)>,
    mut workspace_query: Query<
        (Entity, &mut Workspace, &mut Geometry),
        (With<Workspace>, Without<Hidden>),
    >,
    mut commands: Commands,
) {
    if let Ok((screen_entity, screen_geo)) = screen_query.get(trigger.target()) {
        for (workspace_entity, workspace, mut workspace_geo) in workspace_query.iter_mut() {
            if workspace.no_screen && !workspace.hide {
                commands.queue(ConnectCommand::<ScreenAttachWorkspace>::new(
                    screen_entity,
                    workspace_entity,
                ));
                workspace_geo.geometry = screen_geo.geometry;
                break;
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
    workspace_manager: ResMut<WorkspaceManager>,
) {
    graph.foreach_workspaces_mut(|(entity, workspace, screen_list)| {
        let no_screen = screen_list.map(|l| l.is_empty()).unwrap_or(true);
        update!(workspace.no_screen, no_screen);
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
        update!(workspace_info.hide, workspace.no_screen | workspace.hide);
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
        app.add_observer(on_add_screen);
        app.add_observer(resolve_workspace_request);
        app.add_observer(on_new_workspace);
        app.add_observer(on_destroy_workspace);
        app.add_observer(attach_window_to_workspace);
        app.add_systems(
            PreUpdate,
            (
                on_focus_window.run_if(resource_changed::<FocusedWindow>),
                update_workspace_system,
                update_workspace_window_system.after(on_focus_window),
            )
                .in_set(DWayClientSystem::UpdateWorkspace),
        );
    }
}
