use bevy::prelude::*;
use kayak_ui::{prelude::*, widgets::*, KayakUIPlugin};

use crate::widgets::clock::*;

#[derive(Default)]
pub struct DWayPanelPlugin {}
impl Plugin for DWayPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
    }
}
pub fn setup(mut commands: Commands) {}
impl KayakUIPlugin for DWayPanelPlugin {
    fn build(&self, context: &mut KayakRootContext) {
        context.add_widget_data::<DWayPanelProps, DWayPanelStates>();
        context.add_widget_system(
            DWayPanelProps::default().get_name(),
            widget_update::<DWayPanelProps, DWayPanelStates>,
            panel_render,
        );
    }
}

#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayPanelProps {
    
}
impl Widget for DWayPanelProps {}
#[derive(Component, Clone, PartialEq, Default)]
pub struct DWayPanelStates {}

#[derive(Bundle)]
pub struct DWayPanelBundle {
    pub props: DWayPanelProps,
    pub styles: KStyle,
    pub computed_styles: ComputedStyles,
    pub widget_name: WidgetName,
}
impl Default for DWayPanelBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: DWayPanelProps::default().get_name(),
        }
    }
}
pub fn panel_render(
    In((widget_context, entity)): In<(KayakWidgetContext, Entity)>,
    mut commands: Commands,
    query: Query<&DWayPanelStates>,
) -> bool {
    let state_entity = widget_context.use_state(&mut commands, entity, DWayPanelStates::default());
    if let Ok(status) = query.get(state_entity) {
        let parent_id = Some(entity);
        rsx! {
            <BackgroundBundle
                styles={KStyle {
                    background_color: StyleProp::Value(Color::rgba(1.0, 1.0, 1.0, 0.5)),
                    color: StyleProp::Value(Color::rgba(0.0, 0.0, 0.0, 1.0)),
                    layout_type: LayoutType::Row.into(),
                    top: StyleProp::Value(Units::Pixels(4.0)),
                    left: StyleProp::Value(Units::Pixels(4.0)),
                    right: StyleProp::Value(Units::Pixels(4.0)),
                    padding: StyleProp::Value(Edge::axis( Units::Pixels(2.0) , Units::Pixels(16.0) )),
                    // width: StyleProp::Value(Units::Stretch(1.0)),
                    height: StyleProp::Value(Units::Pixels(32.0)),
                    border_radius: StyleProp::Value(Corner::all(100.0)),

                    ..Default::default()
                }}
            >
                <ClockBundle props={Clock{format:"%B-%e %A %H:%M:%S".to_string()}} />
            </BackgroundBundle>
        };
    }
    true
}
