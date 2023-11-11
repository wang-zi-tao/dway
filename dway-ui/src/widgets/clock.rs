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
Clock
#[derive(Reflect,Default)]{text:String}=>
    {
        let date = chrono::Local::now().naive_local();
        let date_string = date.format(&prop.format).to_string();
        update_state!(text = date_string);
    }
    <NodeBundle>
        <TextBundle
        Text=(Text::from_section(
            &state.text,
            TextStyle {
                font_size: 40.0,
                color: Color::WHITE,
                ..default()
            },
        ))
        />
    </NodeBundle>
);

impl Default for ClockBundle {
    fn default() -> Self {
        Self {
            prop: Default::default(),
            state: Default::default(),
            widget: Default::default(),
            node: Default::default(),
        }
    }
}

pub struct ClockUiPlugin;
impl Plugin for ClockUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clock_render.in_set(ClockSystems::Render));
    }
}
