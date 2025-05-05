use std::marker::PhantomData;

use bevy::{
    core::FrameCount,
    ecs::{
        query::{QueryFilter, WorldQuery},
        system::{lifetimeless::SRes, StaticSystemParam, SystemParam, SystemParamItem},
    },
};
use imports::{QueryData, QueryItem};

use super::{NoTheme, ThemeComponent, ThemeDispatch};
use crate::prelude::*;

#[derive(Component, Default, Debug, Reflect)]
pub struct ClassName(pub String);

#[derive(Component, Default)]
pub struct NoWidgetTheme<T>(PhantomData<T>);

#[derive(Event)]
pub enum ThemeRequestEvent {
    Apply(Entity),
    UnApply(Entity),
    RegisterToGlobal,
    UnRegisterToGlobal,
}

pub trait WidgetInsertObserver<Widget: Component>: Component {
    type Params: SystemParam + 'static;
    type ItemQuery: QueryData + 'static;
    type Filter: QueryFilter + 'static;

    fn on_widget_insert(
        &self,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        params: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    );

    fn register(theme_entity: Entity, world: &mut World) -> WidgetThemeAdapter<Widget>
    where
        Self: Sized,
    {
        let observer = Observer::new(
            move |event: Trigger<'_, OnAdd, Widget>,
                  theme_query: Query<&Self>,
                  params: StaticSystemParam<Self::Params>,
                  mut widget_query: ParamSet<(
                Query<
                    Self::ItemQuery,
                    (
                        Without<NoTheme>,
                        Without<NoWidgetTheme<Widget>>,
                        Self::Filter,
                    ),
                >,
                Query<
                    &mut ThemeComponent,
                    (
                        Without<NoTheme>,
                        Without<NoWidgetTheme<Widget>>,
                        Self::Filter,
                    ),
                >,
            )>,
                  mut commands: Commands| {
                let Ok(this) = theme_query.get(theme_entity) else {
                    return;
                };
                {
                    let mut query = widget_query.p1();
                    let Ok(mut theme_component) = query.get_mut(event.entity()) else {
                        return;
                    };

                    if theme_component.theme_entity != Entity::PLACEHOLDER {
                        return;
                    }
                    theme_component.theme_entity = theme_entity;
                }

                let mut query = widget_query.p0();
                let Ok(query_item) = query.get_mut(event.entity()) else {
                    return;
                };

                let entity_commands = commands.entity(event.entity());
                this.on_widget_insert(
                    theme_entity,
                    query_item,
                    params.into_inner(),
                    entity_commands,
                );
            },
        );
        let entity = world.spawn(observer).set_parent(theme_entity).id();
        WidgetThemeAdapter {
            phantom: std::marker::PhantomData,
            observer: entity,
        }
    }
}

pub trait EventObserver<E, Marker = ()>: Component {
    type Params: SystemParam + 'static;
    type ItemQuery: QueryData + 'static;

    fn on_event(
        &self,
        event: Trigger<UiEvent<E>>,
        theme_entity: Entity,
        query_items: QueryItem<Self::ItemQuery>,
        params: SystemParamItem<Self::Params>,
        commands: EntityCommands,
    );

    fn trigger(
        event: Trigger<UiEvent<E>>,
        theme_query: Query<&Self>,
        mut commands: Commands,
        mut widget_query: ParamSet<(Query<&ThemeComponent>, Query<Self::ItemQuery>)>,
        params: StaticSystemParam<Self::Params>,
    ) where
        Self: Sized,
    {
        let theme_entity = {
            let query = widget_query.p0();
            let Ok(theme_component) = query.get(event.entity()) else {
                return;
            };
            theme_component.theme_entity
        };

        let Ok(this) = theme_query.get(theme_entity) else {
            return;
        };

        let mut query = widget_query.p1();
        let Ok(query_item) = query.get_mut(event.entity()) else {
            return;
        };

        let entity_commands = commands.entity(event.entity());
        Self::on_event(
            &this,
            event,
            theme_entity,
            query_item,
            params.into_inner(),
            entity_commands,
        );
    }
}

pub struct WidgetThemeAdapter<Widget: Component> {
    phantom: std::marker::PhantomData<Widget>,
    pub observer: Entity,
}

pub trait ThemeTrait: Component {
    fn register_to_global(theme_entity: Entity, world: &mut World);
    fn unregister(&self, theme_entity: Entity, world: &mut World);
}

#[derive(Default)]
pub struct GlobalThemePlugin<T: ThemeTrait + Clone> {
    theme: T,
}

impl<T: ThemeTrait + Clone> Plugin for GlobalThemePlugin<T> {
    fn build(&self, app: &mut App) {
        let theme_entity = app.world_mut().spawn(self.theme.clone()).id();
        T::register_to_global(theme_entity, app.world_mut());
    }
}

impl<T: ThemeTrait + Clone> GlobalThemePlugin<T> {
    pub fn new(theme: T) -> Self {
        Self { theme }
    }
}
