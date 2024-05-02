use dway_client_core::navigation::windowstack::WindowStack;
use dway_server::xdg::toplevel::DWayToplevel;
use crate::prelude::*;

#[derive(Component, Debug, Default)]
pub struct WindowTitle;

dway_widget! {
WindowTitle=>
@global(stack: WindowStack -> { state.set_window_entity(stack.focused().unwrap_or(Entity::PLACEHOLDER)); })
@use_state(pub window_entity:Entity=Entity::PLACEHOLDER)
@try_query(window_query:toplevel<-Query<Ref<DWayToplevel>>[*state.window_entity()]->{
    if let Ok(toplevel) = toplevel{
        if !widget.inited || toplevel.is_changed() || state.window_entity_is_changed(){
            state.set_title(toplevel.title.clone().unwrap_or_default());
        }
    } else {
        state.title_mut().clear();
    }
})
@use_state(pub title: String)
@global(theme: Theme)
<MiniNodeBundle>
    <TextBundle Text=(Text::from_section( state.title(),
        TextStyle { font_size: 24.0, color: theme.color("foreground"), font: theme.default_font() },))/>
</MiniNodeBundle>
}

