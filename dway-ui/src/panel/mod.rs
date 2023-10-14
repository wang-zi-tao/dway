use bevy::prelude::*;
use kayak_ui::{prelude::*, widgets::*, KayakUIPlugin};

use crate::widgets::{clock::*, app_entry_list::AppEntryListBundle};

#[derive(Default)]
pub struct DWayPanelPlugin {}
impl Plugin for DWayPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
    }
}
pub fn setup(_commands: Commands) {}
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
pub struct DWayPanelProps {}
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
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    query: Query<&DWayPanelStates>,
) -> bool {
    let state_entity = widget_context.use_state(&mut commands, entity, DWayPanelStates::default());
    if let Ok(_status) = query.get(state_entity) {
        let parent_id = Some(entity);
        rsx! {
            <ElementBundle styles={KStyle {
                position_type: KPositionType::SelfDirected.into(),
                layout_type: LayoutType::Row.into(),
        // width: Units::Pixels(256.0).into(),
                ..Default::default()
            }} >
                <ClockBundle props={Clock{format:"%B-%e %A %H:%M:%S".to_string()}} />
            </ElementBundle>
        };
    }
    true
}
