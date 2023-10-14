use bevy::prelude::*;
use dway_client_core::desktop::{FocusedWindow, CursorOnWindow};
use dway_server::apps::DesktopEntry;
use kayak_ui::{prelude::*, widgets::*};

use crate::{
    create_widget,
    widgets::icon::{Icon, IconBundle},
};

create_widget!(AppEntry, AppEntryPlugin, AppEntryBundle, {
    pub app_entry: Entity,
}, @widget_update widget_update);
impl Default for AppEntry {
    fn default() -> Self {
        Self {
            app_entry: Entity::PLACEHOLDER,
        }
    }
}

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<AppEntry, EmptyState>,
    focus_window: Res<FocusedWindow>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    should_update || focus_window.is_changed()
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    props_query: Query<&AppEntry>,
    windows_query: Query<(&DesktopEntry, Option<&dway_server::apps::WindowList>)>,
    focus_window: Res<FocusedWindow>,
    mut commands: Commands,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        return true;
    };
    let parent_id = Some(entity);
    let size = 48;

    let entry_data = windows_query.get(props.app_entry).ok();
    let is_focus = match (entry_data, &focus_window.0) {
        (Some((_, Some(list))), Some(foucus_window)) => {
            list.contains(foucus_window)
        },
        _ => false,
    };

    let indicate_style = KStyle {
        background_color: if is_focus {
            Color::BLUE.with_a(0.5).into()
        } else {
            Color::BLACK.with_a(0.0).into()
        },
        left: Units::Pixels(8.0).into(),
        right: Units::Pixels(8.0).into(),
        height: Units::Pixels(3.0).into(),
        border_radius: Corner::all(12.0).into(),
        ..Default::default()
    };

    rsx! {
        <BackgroundBundle styles={KStyle{
            left: Units::Pixels(2.0).into(),
            right: Units::Pixels(2.0).into(),
            width: StyleProp::Value(Units::Pixels(44.0)),
            height: StyleProp::Value(Units::Pixels(44.0)),
            border_radius: Corner::all(12.0).into(),
            background_color: Color::rgba(0.2, 0.2, 0.2, 0.5).into(),
            bottom: Units::Pixels(2.0).into(),
            ..Default::default()
        }}>
            <ElementBundle styles={KStyle{
                ..Default::default()
            }}>
                <IconBundle props={{Icon{entity:props.app_entry,size}}}/>
                <BackgroundBundle styles={indicate_style}>
                </BackgroundBundle>
            </ElementBundle>
        </BackgroundBundle>
    };
    true
}
