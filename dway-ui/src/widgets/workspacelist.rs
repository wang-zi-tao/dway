use crate::{
    framework::button::{UiButtonEvent, UiButtonEventKind, UiButtonBundle, UiButton},
    prelude::*,
};
use dway_client_core::workspace::{ScreenList, Workspace, ScreenAttachWorkspace};

#[derive(Component, Default)]
pub struct WorkspaceListUI;

dway_widget! {
WorkspaceListUI=>
@callback{[UiButtonEvent]
    fn on_click(
        In(event): In<UiButtonEvent>,
        query: Query<(&WorkspaceListUISubStateList,&WorkspaceListUISubWidgetList)>,
        mut commands: Commands,
    ) {
        let Ok((state,widget)) = query.get(event.receiver) else {return};
        if event.kind == UiButtonEventKind::Released{
            if let Some(screen) = state.screen_list.first() {
                commands
                    .entity(*screen)
                    .disconnect_all::<ScreenAttachWorkspace>()
                    .connect_to::<ScreenAttachWorkspace>(widget.data_entity);
            }
        }
    }
}
@state_reflect()
@global(theme:Theme)
<MiniNodeBundle @id="List" @style="align-items:center"
    @for_query((workspace,screen_list) in Query<(Ref<Workspace>,Ref<ScreenList>)>::iter()=>[
        workspace=>{state.set_name(workspace.name.clone());},
        screen_list=>{
            state.set_is_focused(screen_list.len()>0);
            state.set_screen_list(screen_list.iter().collect());
        }
    ])>
    <MiniNodeBundle @style="p-4"
        @state_reflect()
        @use_state(pub name:String)
        @use_state(pub is_focused:bool)
        @use_state(pub screen_list:Vec<Entity>)
    >
        <(UiButtonBundle::from(UiButton::new(this_entity,on_click)))
            @material(UiCircleMaterial=>(theme.color("blue"),8.0).into())
            Style=(Style{
                width:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                height:Val::Px(if *state.is_focused() {12.0}else{8.0}),
                ..default()
            }) >
        </UiButtonBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
