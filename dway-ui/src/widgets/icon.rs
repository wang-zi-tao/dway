use bevy::prelude::*;
use dway_server::apps::icon::{IconLoader, IconResorce};
use kayak_ui::{
    prelude::*,
    widgets::{KImage, KImageBundle, KSvg, KSvgBundle, ElementBundle},
};

#[derive(bevy::prelude::Component, Clone, PartialEq, Eq, Debug)]
pub struct Icon {
    pub entity: Entity,
    pub size: u16,
}

impl kayak_ui::prelude::Widget for Icon {}

#[derive(Default)]
pub struct IconPlugin;

impl kayak_ui::KayakUIPlugin for IconPlugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<Icon, kayak_ui::prelude::EmptyState>();
        context.add_widget_system(
            kayak_ui::prelude::WidgetName(std::any::type_name::<Icon>().into()),
            kayak_ui::prelude::widget_update::<Icon, kayak_ui::prelude::EmptyState>,
            render,
        );
    }
}
#[derive(bevy::prelude::Bundle)]
pub struct IconBundle {
    pub props: Icon,
    pub styles: kayak_ui::prelude::KStyle,
    pub computed_styles: kayak_ui::prelude::ComputedStyles,
    pub widget_name: kayak_ui::prelude::WidgetName,
    pub children: KChildren,
}

impl Default for IconBundle {
    fn default() -> Self {
        Self {
            props: Icon {
                entity: Entity::PLACEHOLDER,
                size: 16,
            },
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: kayak_ui::prelude::WidgetName(std::any::type_name::<Icon>().into()),
            children: Default::default(),
        }
    }
}

pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    mut commands: Commands,
    mut props_query: Query<&Icon>,
    mut icon_query: Query<&mut dway_server::apps::icon::Icon>,
    mut icons_loader: ResMut<IconLoader>,
    mut assets: ResMut<AssetServer>,
) -> bool {
    let parent_id = Some(entity);
    if let Ok(prop) = props_query.get_mut(entity) {
        rsx!{
            <ElementBundle> {
                if let Ok(mut icon) = icon_query.get_mut(prop.entity) {
                    if let Some(resource) = icons_loader.load(&mut icon, prop.size, &mut assets) {
                        match &resource {
                            IconResorce::Image(h) => {
                                constructor! { <KImageBundle image={KImage(h.clone())} /> };
                            }
                            IconResorce::Svg(h) => {
                                constructor! { <KSvgBundle svg={KSvg(h.clone())} /> };
                            }
                        }
                    }
                }
            } </ElementBundle>
        };
    }
    true
}
