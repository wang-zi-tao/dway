use animation::translation::UiTranslationAnimation;
use dway_client_core::{
    desktop::CursorOnScreen,
    workspace::{ScreenAttachWorkspace, ScreenList, Workspace, WorkspaceManager},
};
use util::DwayUiDirection;

use crate::{
    popups::workspace_window_preview::{
        WorkspaceWindowPreviewPopup, WorkspaceWindowPreviewPopupBundle,
    },
    prelude::*,
};

#[derive(Component, Default)]
pub struct WorkspaceListUI;

fn on_click(
    event: UiEvent<UiButtonEvent>,
    query: Query<&WorkspaceListUISubWidgetList>,
    focus_screen: Res<CursorOnScreen>,
    mut commands: Commands,
) {
    let Ok(widget) = query.get(event.receiver()) else {
        return;
    };
    if event.kind == UiButtonEventKind::Released {
        if let Some(screen) = focus_screen.get_screen() {
            commands
                .entity(screen)
                .disconnect_all::<ScreenAttachWorkspace>()
                .connect_to::<ScreenAttachWorkspace>(widget.data_entity);
        }
        commands
            .spawn((
                UiPopup::default(),
                UiTranslationAnimation::new(DwayUiDirection::TOP),
                AnimationTargetNodeState(style!("absolute top-42 align-self:center")),
            ))
            .with_children(|c| {
                c.spawn((
                    WorkspaceWindowPreviewPopup {
                        workspace: widget.data_entity,
                        ..Default::default()
                    },
                    style!("h-auto w-auto"),
                ));
            })
            .set_parent(event.receiver());
    }
}

dway_widget! {
WorkspaceListUI=>
@add_callback{[UiEvent<UiButtonEvent>]on_click}
@state_reflect()
@global(theme: Theme)
@global(workspace_manager: WorkspaceManager)
<Node @id="List" @style="align-items:center"
    @for_query((workspace,screen_list) in Query<(Ref<Workspace>,Ref<ScreenList>)>
        ::iter_many(workspace_manager.workspaces.iter().cloned())=>[
        workspace=>{state.set_name(workspace.name.clone());},
        screen_list=>{ state.set_is_focused(screen_list.len()>0); }
    ])
    @material(RoundedUiRectMaterial=>rounded_rect(theme.color("background1"), 12.0))
>
    <Node @id="ws" @style="flex-col"
        @state_reflect()
        @use_state(pub name:String)
        @use_state(pub is_focused:bool)
        @use_state(pub screen_list:Vec<Entity>)
    >
        <UiButton NoTheme @style="w-16 h-16 align-items:center justify-items:center" @on_event(on_click) >
            <Node
                @material(UiCircleMaterial=>circle_material(theme.color("blue")))
                Node=(Node{
                    width:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                    height:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                    ..default()
                }) >
            </Node>
        </UiButton>
    </Node>
</Node>
}
