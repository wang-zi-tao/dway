use bevy::prelude::*;
use chrono::Local;
use dway_server::components::WindowMark;
use kayak_ui::{
    prelude::*,
    widgets::{TextProps, TextWidgetBundle},
    KayakUIPlugin,
};

#[derive(Default)]
pub struct DWayClockPlugin {}
impl Plugin for DWayClockPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_system(update);
    }
}
pub fn widget_update2<
    Props: PartialEq + Component + Clone,
    State: PartialEq + Component + Clone,
>(
    In((widget_context, entity, previous_entity)): In<(KayakWidgetContext, Entity, Entity)>,
    widget_param: WidgetParam<Props, State>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    // dbg!(should_update);
    should_update
}

impl KayakUIPlugin for DWayClockPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<Clock, ClockState>();
        context.add_widget_system(
            Clock::default().get_name(),
            widget_update::<Clock, ClockState>,
            render,
        );
    }
}
pub fn update(mut clock_states: Query<&mut ClockState, Without<PreviousWidget>>) {
    for mut state in clock_states.iter_mut() {
        let date = Local::now().naive_local();
        state.time = date.format(&state.format).to_string();
    }
}

#[derive(Component, Clone, PartialEq, Eq)]
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
impl Widget for Clock {}
#[derive(Debug, Component, Clone, PartialEq, Eq)]
pub struct ClockState {
    format: String,
    time: String,
}
impl Default for ClockState {
    fn default() -> Self {
        Self {
            format: "".into(),
            time: "".into(),
            // date: Local::now(),
        }
    }
}
#[derive(Bundle)]
pub struct ClockBundle {
    pub props: Clock,
    pub styles: KStyle,
    pub computed_styles: ComputedStyles,
    pub widget_name: WidgetName,
}
impl Default for ClockBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: Clock::default().get_name(),
        }
    }
}
pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    props_query: Query<&Clock>,
    state_query: Query<&ClockState>,
) -> bool {
    let props = props_query.get(entity).unwrap();
    let parent_id = Some(entity);
    let state_entity = widget_context.use_state(
        &mut commands,
        entity,
        ClockState {
            format: props.format.clone(),
            // date: Local::now(),
            ..Default::default()
        },
    );
    let date = state_query
        .get(state_entity)
        .map(|s| &*s.time)
        .unwrap_or_else(|_| "")
        .to_string();
    // let date = date.format(&props.format).to_string();
    rsx! {
        <TextWidgetBundle
            text={TextProps {
                content: date,
                size: 20.0,
                ..Default::default()
            }}
            styles={KStyle{
                left: Units::Stretch(0.5).into(),
                right: Units::Stretch(0.5).into(),
                ..Default::default()
            }}
        />
    };
    true
}
