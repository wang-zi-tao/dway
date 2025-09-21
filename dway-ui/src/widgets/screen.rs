use dway_client_core::{screen::ScreenWindowList, UiAttachData};
use dway_server::{geometry::GlobalGeometry, util::rect::IRect};

use super::window::WindowUI;
use crate::prelude::*;

#[derive(Component, Reflect)]
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
    app.register_type::<ScreenWindows>();
    app.register_type::<ScreenWindowsState>();
    app.register_type::<ScreenWindowsSubStateWindows>();
}
@state_reflect()
@use_state(pub window_list: Vec<Entity>)
@use_state(pub screen_geometry: IRect)
@query(screen_query: (global_geo, window_list )<-Query<(Ref<GlobalGeometry>, Option<Ref<ScreenWindowList>>)>[prop.screen]->{
    let init = !widget.inited || prop.is_changed();
    if !init {
        commands.queue(ConnectCommand::<UiAttachData>::new(this_entity, prop.screen));
    }
    if !init || window_list.as_ref().map(|l|l.is_changed()).unwrap_or(false) {
        state.set_window_list(window_list.iter().flat_map(|l|l.iter()).collect());
    }
    if !widget.inited || global_geo.is_changed(){
        state.set_screen_geometry(global_geo.geometry);
    }
})
<Node @id="Windows" @style="full absolute"
    @map(*window_entity:Entity <= window_entity in state.window_list().iter().cloned() => {
        state.set_window_entity(window_entity);
    })>
    <(WindowUI{
        window_entity:*state.window_entity(),
        screen_geomety: *root_state.screen_geometry()
    }) 
        @style="absolute full" @use_state(window_entity:Entity=Entity::PLACEHOLDER)
        @state_component(#[derive(Reflect)])
    />
</Node>
}
