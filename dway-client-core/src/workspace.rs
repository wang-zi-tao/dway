use crate::prelude::*;

#[derive(Component, Reflect)]
pub struct Workspace {
    pub name: String,
}

relationship!(WindowOnWorkspace=>WindowWorkspaceList>-<WindowList);
relationship!(WorkspaceOnScreen=>ScreenWorkspaceList>-<ScreenList);

pub struct WorkspacePlugin;
impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.register_relation::<WindowOnWorkspace>();
        app.register_relation::<WorkspaceOnScreen>();
        app.register_type::<Workspace>();
    }
}
