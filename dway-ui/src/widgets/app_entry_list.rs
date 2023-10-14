use bevy::prelude::*;
use dway_server::apps::DesktopEntry;
use kayak_ui::prelude::*;
use kayak_ui::widgets::*;

use crate::create_widget;
use crate::widgets::app_entry::AppEntry;
use crate::widgets::app_entry::AppEntryBundle;

create_widget!(AppEntryList, AppEntryListPlugin, AppEntryListBundle, {

}, @widget_update widget_update);
impl Default for AppEntryList {
    fn default() -> Self {
        Self {}
    }
}

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<AppEntryList, EmptyState>,
    entry_query: Query<Entity, (With<DesktopEntry>, Changed<dway_server::apps::WindowList>)>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    should_update || !entry_query.is_empty()
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    entry_query: Query<
        (Entity, Option<&dway_server::apps::WindowList>),
        (With<DesktopEntry>, With<dway_server::apps::WindowList>),
    >,
    mut commands: Commands,
) -> bool {
    let parent_id = Some(entity);
    rsx! {
        <ElementBundle styles={KStyle{
            layout_type: LayoutType::Row.into(),
            width: Units::Auto.into(),
            col_between: Units::Pixels(4.0).into(),
            padding_top:Units::Pixels(2.0).into(),
            padding_bottom:Units::Pixels(2.0).into(),
            ..Default::default()
        }}> 
        {
          entry_query.for_each(|(entity, windows)| {
              if windows.is_some_and(|l| !l.is_empty()) {
                  constructor! {
                    <ElementBundle styles={KStyle{
                        min_width: StyleProp::Value(Units::Pixels(44.0)),
                        min_height: StyleProp::Value(Units::Pixels(44.0)),
                        ..Default::default()
                    }}> 
                        <AppEntryBundle props={{AppEntry{app_entry:entity}}} />
                    </ElementBundle>
                  };
              }
          });
        }
        </ElementBundle>
    };
    true
}
