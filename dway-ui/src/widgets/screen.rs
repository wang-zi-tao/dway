use dway_client_core::workspace::{ScreenWorkspaceList, WindowList};
use dway_server::geometry::GlobalGeometry;
use crate::{prelude::*, util::irect_to_style};
use super::window::{WindowUI, WindowUIBundle};
use dway_server::util::rect::IRect;

#[derive(Component)]
pub struct ScreenWindows {
    pub screen: Entity,
}
impl Default for ScreenWindows {
    fn default() -> Self {
        Self {
            screen: Entity::PLACEHOLDER,
        }
    }
}

dway_widget! {
ScreenWindows=>
@plugin{
    app.register_type::<ScreenWindowsState>();
    app.register_type::<ScreenWindowsSubStateScreen>();
    app.register_type::<ScreenWindowsSubStateWorkspace>();
    app.register_type::<ScreenWindowsSubStateWindow>();
}
@state_component(#[derive(Reflect,serde::Serialize,serde::Deserialize)])
<MiniNodeBundle @id="Screen" @style="full absolute"
    @for_query(workspace_list in Query<Ref<ScreenWorkspaceList>>::iter()=>[
        workspace_list=>{ state.set_workspace_list(workspace_list.iter().collect());
    }])>
    <MiniNodeBundle @id="Workspace" @style="full absolute"
        @state_component(#[derive(Reflect)]) @use_state(workspace_list:Vec<Entity>)
        @for_query((workspace_rect,window_list)in Query<(Ref<GlobalGeometry>,Ref<WindowList>)>::iter_many(state.workspace_list().iter())=>[
            workspace_rect=>{ state.set_workspace_rect(workspace_rect.geometry); },
            window_list=>{state.set_window_list(window_list.iter().collect());},
        ])>
        <MiniNodeBundle @id="Window" @style="full absolute" @state_component(#[derive(Reflect)])
            @use_state(workspace_rect:IRect) @use_state(pub window_list:Vec<Entity>)
            @map(*window_entity:Entity <= window_entity in state.window_list().iter().cloned() => {
                state.set_window_entity(window_entity);
            })>
            <WindowUIBundle @style="absolute full" @use_state(window_entity:Entity=Entity::PLACEHOLDER)
                @state_component(#[derive(Reflect)])
                WindowUI=(WindowUI{
                    window_entity:*state.window_entity(),
                    workspace_entity:workspace_widget.data_entity,
                    screen_entity:screen_widget.data_entity,
                    workspace_rect:*workspace_state.workspace_rect(),
                })/>
        </MiniNodeBundle>
    </MiniNodeBundle>
</MiniNodeBundle>
}
