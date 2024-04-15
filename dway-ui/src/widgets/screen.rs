use dway_client_core::screen::ScreenWindowList;

use crate::prelude::*;
use super::window::{WindowUI, WindowUIBundle};

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
    app.register_type::<ScreenWindowsSubStateWindows>();
}
@state_component(#[derive(Reflect,serde::Serialize,serde::Deserialize)])
@use_state(pub window_list: Vec<Entity>)
@component(window_list<-Query<Ref<ScreenWindowList>>[prop.screen]->{
    state.set_window_list(window_list.iter().collect()); 
})
<MiniNodeBundle @id="Windows" @style="full absolute"
    @map(*window_entity:Entity <= window_entity in state.window_list().iter().cloned() => {
        state.set_window_entity(window_entity);
    })>
    <WindowUIBundle @style="absolute full" @use_state(window_entity:Entity=Entity::PLACEHOLDER)
        @state_component(#[derive(Reflect)])
        WindowUI=(WindowUI{ window_entity:*state.window_entity(), })
    />
</MiniNodeBundle>
}
