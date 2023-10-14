use bevy::prelude::*;
use dway_server::apps::DesktopEntry;
use kayak_ui::prelude::*;
use kayak_ui::widgets::*;

use crate::create_render;
use crate::create_widget;
use crate::widgets::app_entry_list::AppEntryListBundle;

create_widget!(Dock, DockPlugin, DockBundle, {});
impl Default for Dock {
    fn default() -> Self {
        Self {  }
    }
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    props_query: Query<&Dock>,
    mut commands: Commands,
    mut assets: ResMut<AssetServer>,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        return true;
    };
    let parent_id = Some(entity);
    rsx! {
    <ElementBundle styles={KStyle {
        position_type: KPositionType::SelfDirected.into(),
        layout_type: LayoutType::Row.into(),
// width: Units::Pixels(256.0).into(),
        ..Default::default()
    }} >
        <AppEntryListBundle />
    </ElementBundle>
    };
    true
}
