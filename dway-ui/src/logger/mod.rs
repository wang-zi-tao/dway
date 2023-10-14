use crate::create_widget;
use bevy::prelude::*;
use kayak_ui::{
    prelude::*,
    widgets::{TextProps, TextWidgetBundle},
};

create_widget!(Logger,LoggerPlugin,LoggerBundle,{ });
impl Default for Logger{
    fn default() -> Self {
        Self {  }
    }
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
) -> bool {
    let parent_id = Some(entity);
    rsx! {
        <TextWidgetBundle
            text={TextProps {
                // content: date,
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
