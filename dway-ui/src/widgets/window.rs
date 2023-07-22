use bevy::{
    input::{mouse::MouseButtonInput, ButtonState},
    prelude::*,
    render::texture::{BevyDefault, ImageSampler, TextureFormatPixelInfo},
};
use dway_client_core::desktop::{FocusedWindow, WindowStack};
use dway_server::{geometry::GlobalGeometry, wl::surface::WlSurface, xdg::XdgSurface};
use kayak_ui::{
    prelude::{
        rsx, EventType, KChildren, KEvent, KPositionType, KStyle, KayakWidgetContext, OnEvent,
        StyleProp, Units, WidgetParam,
    },
    widgets::{
        BackgroundBundle, ElementBundle, KButton, KButtonBundle, KImage, KImageBundle,
        KayakAppBundle,
    },
    KayakUIPlugin,
};

#[derive(Default)]
pub struct DWayWindowPlugin {}
impl KayakUIPlugin for DWayWindowPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<Window, WindowState>();
        context.add_widget_system(
            kayak_ui::prelude::WidgetName(std::any::type_name::<Window>().into()),
            widget_update,
            render,
        );
    }
}

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<Window, WindowState>,
    window_query: Query<
        Entity,
        Or<(
            Changed<XdgSurface>,
            Changed<GlobalGeometry>,
            Changed<WlSurface>,
        )>,
    >,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    let props = widget_param.props_query.get(entity).unwrap();
    should_update || window_query.contains(props.entity)
}

#[derive(bevy::prelude::Bundle)]
pub struct WindowBundle {
    pub props: Window,
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
            widget_name: kayak_ui::prelude::WidgetName(std::any::type_name::<Window>().into()),
        }
    }
}

#[derive(Component, Clone, PartialEq, Eq)]
pub struct Window {
    pub entity: Entity,
}

impl Default for Window {
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
    props_query: Query<&Window>,
    state_query: Query<&WindowState>,
    window_query: Query<
        (&XdgSurface, &GlobalGeometry, &WlSurface),
        // With<WindowMark>,
    >,
    mut assets: ResMut<Assets<Image>>,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        error!("no props");
        return true;
    };
    let state_entity = widget_context.use_state(&mut commands, entity, WindowState::default());
    let Ok((geometry, rect, surface)) = window_query.get(props.entity) else {
        error!("surface error component {:?}", props.entity);
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
    // let geo_style = KStyle {
    //     position_type: StyleProp::Value(KPositionType::SelfDirected),
    //     left: StyleProp::Value(Units::Pixels(rect.loc.x as f32)),
    //     top: StyleProp::Value(Units::Pixels(rect.loc.x as f32)),
    //     width: StyleProp::Value(Units::Pixels(rect.size.w as f32)),
    //     height: StyleProp::Value(Units::Pixels(rect.size.h as f32)),
    //     background_color: Color::rgba_u8(255, 0, 0, 64).into(),
    //     ..Default::default()
    // };
    let parent_id = Some(entity);
    let background_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
        position_type: KPositionType::SelfDirected.into(),
        // background_color: Color::WHITE.into(),
        ..Default::default()
    };

    let backend_entity = props.entity;
    // let surface_id_clone = surface_id.clone();
    let on_event = OnEvent::new(
        move |In(_entity): In<Entity>,
              mut event: ResMut<KEvent>,
              mut state_query: Query<&mut WindowState>,
              // mut mouse_button_events: EventWriter<MouseButtonOnWindow>,
              // mut keyboard_events: EventWriter<KeyboardInputOnWindow>,
              mut stack: ResMut<WindowStack>,
              mut output_focus: ResMut<FocusedWindow>,
              // mut mouse_events:Res<InputEvent<MouseButton>>,
        | {
            let mut state = state_query.get_mut(state_entity).unwrap();
            match event.event_type {
                EventType::MouseIn(c) => {
                    state.mouse_in_rect=true;
                    output_focus.0=Some(backend_entity);
                }
                EventType::MouseOut(c) => {
                    state.mouse_in_rect=false;
                }
                _ => { }
            };
        },
    );
    rsx! {
        <ElementBundle styles={root_style.clone()} >
            <BackgroundBundle
                // image={KImage( image )}
                styles={bbox_style.clone()}
            />
            <KImageBundle
                // image={KImage( image )}
                image={KImage(surface.image.clone())}
                styles={bbox_style}
            />
            // <ElementBundle
            //     // styles={geo_style.clone()}
            //         on_event={on_event}
            // >
            // </ElementBundle>
        </ElementBundle>
    };
    true
}
