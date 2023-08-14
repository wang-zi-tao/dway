use crate::create_widget;
use crate::widgets::window::{ WindowBundle, WindowUI};
use bevy::prelude::*;
use dway_server::events::{Destroy, Insert};
use dway_server::wl::surface::WlSurface;
use dway_server::xdg::{XdgSurface, DWayWindow};
use dway_server::xdg::toplevel::XdgToplevel;
use kayak_ui::widgets::{BackgroundBundle, TextProps, TextWidgetBundle};
use kayak_ui::{prelude::*, widgets::ElementBundle};

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<WindowUI, EmptyState>,
    create_window_events: EventReader<Insert<DWayWindow>>,
    destroy_window_events: EventReader<Destroy<DWayWindow>>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    should_update || !create_window_events.is_empty() || !destroy_window_events.is_empty()
}

create_widget!(WindowArea, WindowAreaPlugin, WindowAreaBundle, {},@widget_update widget_update);
pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    windows_query: Query<Entity, With<DWayWindow>>,
) -> bool {
    let parent_id = Some(entity);
    let background_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        position_type: KPositionType::SelfDirected.into(),
        background_color: Color::rgba_u8(0, 0, 0, 0).into(),
        ..Default::default()
    };
    rsx! {
      <ElementBundle styles={background_style.clone()} > {
        windows_query.iter().for_each(|entity|{
          constructor!{
            <ElementBundle styles={background_style.clone()}>
                <WindowBundle
                  props = {WindowUI{entity}}
                />
            </ElementBundle>
          }
        })
      }</ElementBundle>
    };
    true
}
