use bevy::prelude::*;
use dway_client_core::desktop::{FocusedWindow, WindowStack};
use dway_server::{
    geometry::GlobalGeometry,
    macros::Connectable,
    try_get,
    wl::surface::WlSurface,
    xdg::{popup::XdgPopup, DWayWindow, PopupList},
};
use kayak_ui::{
    prelude::{
        constructor, rsx, EventType, KChildren, KEvent, KPositionType, KStyle, KayakWidgetContext,
        OnEvent, StyleProp, Units, WidgetParam,
    },
    widgets::{BackgroundBundle, ElementBundle, KImage, KImageBundle},
    KayakUIPlugin,
};

#[derive(Default)]
pub struct DWayWindowPlugin {}
impl KayakUIPlugin for DWayWindowPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<WindowUI, WindowState>();
        context.add_widget_system(
            kayak_ui::prelude::WidgetName(std::any::type_name::<WindowUI>().into()),
            widget_update,
            render,
        );
    }
}

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<WindowUI, WindowState>,
    window_query: Query<
        Entity,
        Or<(
            Changed<DWayWindow>,
            Changed<GlobalGeometry>,
            Changed<WlSurface>,
            Changed<PopupList>,
        )>,
    >,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    let props = widget_param.props_query.get(entity).unwrap();
    should_update || window_query.contains(props.entity)
}

#[derive(bevy::prelude::Bundle)]
pub struct WindowBundle {
    pub props: WindowUI,
    pub styles: kayak_ui::prelude::KStyle,
    pub computed_styles: kayak_ui::prelude::ComputedStyles,
    pub widget_name: kayak_ui::prelude::WidgetName,
}
impl Default for WindowBundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: kayak_ui::prelude::WidgetName(std::any::type_name::<WindowUI>().into()),
        }
    }
}

#[derive(Component, Reflect, FromReflect, Clone, PartialEq, Eq)]
pub struct WindowUI {
    pub entity: Entity,
}

impl Default for WindowUI {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
        }
    }
}
#[derive(Component, Default, Clone, PartialEq, Eq)]
pub struct WindowState {
    focused: bool,
    mouse_in_rect: bool,
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    props_query: Query<&WindowUI>,
    _state_query: Query<&WindowState>,
    window_query: Query<(&GlobalGeometry, &WlSurface, Option<&PopupList>), With<DWayWindow>>,
    popup_query: Query<Entity, With<XdgPopup>>,
    _assets: ResMut<Assets<Image>>,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        error!("no props");
        return true;
    };
    let state_entity = widget_context.use_state(&mut commands, entity, WindowState::default());
    let Some((rect, surface, popups)) = try_get!(window_query, props.entity) else {
        return true;
    };
    // let bbox_loc = rect.0.loc + offset.0.loc;
    let image_rect = surface.image_rect().offset(rect.pos());
    let root_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        // background_color:Color::rgba_u8(255, 0, 0, 64).into(),
        position_type: KPositionType::SelfDirected.into(),
        ..Default::default()
    };
    let bbox_style = KStyle {
        left: StyleProp::Value(Units::Pixels(image_rect.pos().x as f32)),
        top: StyleProp::Value(Units::Pixels(image_rect.pos().y as f32)),
        width: StyleProp::Value(Units::Pixels(image_rect.size().x as f32)),
        height: StyleProp::Value(Units::Pixels(image_rect.size().y as f32)),
        background_color: Color::WHITE.with_a(0.1).into(),
        position_type: KPositionType::SelfDirected.into(),
        ..Default::default()
    };
    let parent_id = Some(entity);
    let _background_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        position_type: KPositionType::SelfDirected.into(),
        // background_color: Color::WHITE.into(),
        ..Default::default()
    };

    let backend_entity = props.entity;
    let _on_event = OnEvent::new(
        move |In(_entity): In<Entity>,
              event: ResMut<KEvent>,
              mut state_query: Query<&mut WindowState>,
              _stack: ResMut<WindowStack>,
              mut output_focus: ResMut<FocusedWindow>| {
            let mut state = state_query.get_mut(state_entity).unwrap();
            match event.event_type {
                EventType::MouseIn(_c) => {
                    state.mouse_in_rect = true;
                    output_focus.0 = Some(backend_entity);
                }
                EventType::MouseOut(_c) => {
                    state.mouse_in_rect = false;
                }
                _ => {}
            };
        },
    );
    rsx! {
        <ElementBundle styles={root_style} >
            <BackgroundBundle
                styles={bbox_style.clone()}
            />
            <KImageBundle
                image={KImage(surface.image.clone())}
                styles={bbox_style.clone()}
            ></KImageBundle>
            {
                if let Some(popups)=popups {
                    for popup_entity in popup_query.iter_many(popups.iter()){
                        constructor!{
                            <WindowBundle
                              props = {WindowUI{entity: popup_entity}}
                            />
                        }
                    }
                }
            }
        </ElementBundle>
    };
    true
}
