use dway_server::{
    geometry::{Geometry, GlobalGeometry},
    util::rect::IRect,
    xdg::DWayWindow,
};

use crate::{
    prelude::*,
    screen::{create_screen, Screen},
    window::Hidden,
};

#[derive(Component, Default, Debug, Reflect)]
pub struct WorkspaceSet;

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
    new_window: Query<(Entity, &GlobalGeometry), (With<DWayWindow>, Without<WindowWorkspaceList>)>,
    workspace_query: Query<(Entity, &GlobalGeometry), (With<Workspace>, Without<Hidden>)>,
    mut commands: Commands,
) {
    new_window.for_each(|(window, window_geo)| {
        for (workspace, workspace_geo) in workspace_query.iter() {
            if workspace_geo.intersection(window_geo.geometry).size() != IVec2::default() {
                commands.add(ConnectCommand::<WindowOnWorkspace>::new(window, workspace));
            }
        }
    });
}

pub fn attach_workspace_to_screen(
    screen_query: Query<(Entity, &GlobalGeometry), (With<Screen>, Without<ScreenWorkspaceList>)>,
    mut workspace_query: Query<
        (Entity, &mut Geometry, Option<&ScreenList>),
        (With<Workspace>, Without<Hidden>),
    >,
    mut commands: Commands,
) {
    screen_query.for_each(|(screen_entity, screen_geo)| {
        for (workspace_entity, mut workspace_geo, screens) in workspace_query.iter_mut() {
            if screens.map(|s| s.len() == 0).unwrap_or(true) {
                commands.add(ConnectCommand::<ScreenAttachWorkspace>::new(
                    screen_entity,
                    workspace_entity,
                ));
                workspace_geo.geometry = screen_geo.geometry;
                return;
            }
        }
        let geo = Geometry {
            geometry: IRect::from_pos_size(IVec2::default(), screen_geo.size()),
        };
        let workspace_entity = commands
            .spawn(WorkspaceBundle {
                global: screen_geo.add(&geo),
                geo,
                ..Default::default()
            })
            .id();
        commands.add(ConnectCommand::<ScreenAttachWorkspace>::new(
            screen_entity,
            workspace_entity,
        ));
    });
}

pub struct WorkspacePlugin;
impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowOnWorkspace>();
        app.register_relation::<ScreenAttachWorkspace>();
        app.register_type::<Workspace>();
        app.add_systems(
            PreUpdate,
            (
                attach_workspace_to_screen
                    .in_set(DWayClientSystem::CreateComponent)
                    .after(create_screen),
                attach_window_to_workspace
                    .in_set(DWayClientSystem::CreateComponent)
                    .after(attach_workspace_to_screen),
            ),
        );
    }
}
