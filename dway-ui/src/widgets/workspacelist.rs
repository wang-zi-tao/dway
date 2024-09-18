use dway_client_core::{
    desktop::CursorOnScreen,
    workspace::{ScreenAttachWorkspace, ScreenList, Workspace, WorkspaceManager},
};
use dway_ui_framework::widgets::button::UiRawButtonBundle;

use crate::prelude::*;

#[derive(Component, Default)]
pub struct WorkspaceListUI;

fn on_click(
    In(event): In<UiButtonEvent>,
    query: Query<&WorkspaceListUISubWidgetList>,
    focus_screen: Res<CursorOnScreen>,
    mut commands: Commands,
) {
    let Ok(widget) = query.get(event.receiver) else {
        return;
    };
    match event.kind {
        UiButtonEventKind::Released => {
            if let Some(screen) = focus_screen.get_screen() {
                commands
                    .entity(screen)
                    .disconnect_all::<ScreenAttachWorkspace>()
                    .connect_to::<ScreenAttachWorkspace>(widget.data_entity);
            }
        }
        UiButtonEventKind::Hovered => {}
        _ => {}
    }
}

dway_widget! {
WorkspaceListUI=>
@add_callback{[UiButtonEvent]on_click}
@state_reflect()
@global(theme: Theme)
@global(workspace_manager: WorkspaceManager)
<MiniNodeBundle @id="List" @style="align-items:center"
    @for_query((workspace,screen_list) in Query<(Ref<Workspace>,Ref<ScreenList>)>
        ::iter_many(workspace_manager.workspaces.iter().cloned())=>[
        workspace=>{state.set_name(workspace.name.clone());},
        screen_list=>{ state.set_is_focused(screen_list.len()>0); }
    ])>
    <MiniNodeBundle @id="ws"
        @state_reflect()
        @use_state(pub name:String)
        @use_state(pub is_focused:bool)
        @use_state(pub screen_list:Vec<Entity>)
    >
        <(UiRawButtonBundle::from(UiButton::new(node!(ws),on_click))) @style="p-4">
            <MiniNodeBundle
                @material(UiCircleMaterial=>circle_material(theme.color("blue")))
                Style=(Style{
                    width:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                    height:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                    ..default()
                }) >
            </MiniNodeBundle>
        </UiRawButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
