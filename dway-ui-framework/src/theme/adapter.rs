use bevy::{
    core::FrameCount,
    ecs::{
        query::WorldQuery,
        system::{lifetimeless::SRes, StaticSystemParam, SystemParam, SystemParamItem},
    },
};
use imports::{QueryData, QueryItem};

use super::ThemeDispatch;
use crate::prelude::*;

#[derive(Component, Default, Debug, Reflect)]
pub struct ClassName(pub String);

pub trait WidgetTheme<Widget: Component>: Component {
    type Params: SystemParam + 'static;
    type ItemQuery: QueryData + 'static;

    fn apply(
        &self,
        widget: &mut Widget,
        query_items: QueryItem<Self::ItemQuery>,
        params: SystemParamItem<Self::Params>,
        commands: &mut EntityCommands,
    );

    fn register(&self, theme_entity: Entity, commands: &mut Commands) -> WidgetThemeAdapter<Widget>
    where
        Self: Sized,
    {
        let observer = Observer::new(
            move |event: Trigger<'_, OnAdd, Widget>,
                  theme_query: Query<&Self>,
                  params: StaticSystemParam<Self::Params>,
                  mut widget_query: Query<(&mut Widget, Self::ItemQuery)>,
                  mut commands: Commands| {
                let Ok(this) = theme_query.get(theme_entity) else {
                    return;
                };
                let Ok((mut widget, query_item)) = widget_query.get_mut(event.entity()) else {
                    return;
                };
                this.apply(
                    &mut widget,
                    query_item,
                    params.into_inner(),
                    &mut commands.entity(event.entity()),
                );
            },
        );
        let entity = commands.spawn(observer).set_parent(theme_entity).id();
        WidgetThemeAdapter {
            phantom: std::marker::PhantomData,
            observer: entity,
        }
    }
}

pub struct WidgetThemeAdapter<Widget: Component> {
    phantom: std::marker::PhantomData<Widget>,
    pub observer: Entity,
}
