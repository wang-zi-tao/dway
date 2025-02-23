use crate::prelude::*;

#[derive(Component, SmartDefault)]
pub struct Clock {
    #[default("%B-%e  %H:%M:%S %A".to_string())]
    pub format: String,
}

dway_widget!{ 
Clock=>
@use_state{ pub text:String="".to_string() }
@state_component{#[derive(Debug)]}
@before{
    let date = chrono::Local::now().naive_local();
    let date_string = date.format(&prop.format).to_string();
    if state.text() != &date_string{ state.set_text(date_string); }
}
@global(theme:Theme)
<Node
    Text=(Text::new(state.text()))
    TextFont=(theme.text_font(24.0))
    TextColor=(theme.color("panel-foreground").into())
/> 
}
