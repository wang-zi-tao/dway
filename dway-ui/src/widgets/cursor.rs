use bevy::prelude::Query;
use bevy::prelude::*;
use dway_server::{geometry::GlobalGeometry, input::pointer::WlPointer, wl::surface::WlSurface};
use kayak_ui::{
    prelude::{
        rsx, EmptyState, KChildren, KPositionType, KStyle, KayakWidgetContext, StyleProp, Units,
        WidgetParam,
    },
    widgets::{ElementBundle, KImage, KImageBundle},
};

use crate::create_widget;

#[derive(bevy::prelude::Component, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub entity: Entity,
}
impl Default for Cursor {
    fn default() -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
        }
    }
}

impl kayak_ui::prelude::Widget for Cursor {}

#[derive(Default)]
pub struct CursorPlugin;

impl kayak_ui::KayakUIPlugin for CursorPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<Cursor, kayak_ui::prelude::EmptyState>();
        context.add_widget_system(
            kayak_ui::prelude::WidgetName(std::any::type_name::<Cursor>().into()),
            (kayak_ui::prelude::widget_update::<Cursor, kayak_ui::prelude::EmptyState>),
            render,
        );
    }
}
#[derive(bevy::prelude::Bundle, Default)]
pub struct CursorBundle {
    pub props: Cursor,
    pub styles: kayak_ui::prelude::KStyle,
    pub computed_styles: kayak_ui::prelude::ComputedStyles,
    pub widget_name: kayak_ui::prelude::WidgetName,
}

pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<Cursor, EmptyState>,
    cursor_query: Query<Entity, Or<(Changed<GlobalGeometry>, Changed<WlSurface>)>>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    let props = widget_param.props_query.get(entity).unwrap();
    should_update || cursor_query.contains(props.entity)
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    props_query: Query<&Cursor>,
    pointer_query: Query<(&GlobalGeometry, &WlSurface), With<WlPointer>>,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        error!("no props");
        return true;
    };
    let Ok((rect, surface)) = pointer_query.get(props.entity) else {
        error!("surface has not components {:?}", props.entity);
        return true;
    };
    let image_rect = surface.image_rect().offset(rect.pos());
    let root_style = KStyle {
        left: StyleProp::Inherit,
        right: StyleProp::Inherit,
        top: StyleProp::Inherit,
        bottom: StyleProp::Inherit,
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
    rsx! {
        <ElementBundle styles={root_style} >
            <KImageBundle
                image={KImage(surface.image.clone())}
                styles={bbox_style.clone()}
            ></KImageBundle>
        </ElementBundle>
    };
    true
}
