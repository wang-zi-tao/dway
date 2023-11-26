use crate::prelude::*;

#[derive(Component)]
pub struct Clock {
    pub format: String,
}
impl Default for Clock {
    fn default() -> Self {
        Self {
            format: "%B-%e  %H:%M:%S %A".to_string(),
        }
    }
}

dway_ui_derive::dway_widget!(
Clock=>
@use_state{ pub text:String="".to_string() }
@state_component{#[derive(Debug)]}
@before{
    let date = chrono::Local::now().naive_local();
    let date_string = date.format(&prop.format).to_string();
    if state.text() != &date_string{
        state.set_text(date_string);
    }
}
    <NodeBundle>
        <TextBundle
        Text=(Text::from_section(
            state.text(),
            TextStyle {
                font_size: 24.0,
                color: Color::WHITE,
                ..default()
            },
        ))
        />
    </NodeBundle>
);

pub struct ClockUiPlugin;
impl Plugin for ClockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clock_render.in_set(ClockSystems::Render));
    }
}
