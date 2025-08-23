use std::marker::PhantomData;

use bevy::ecs::{
    query::{QueryFilter, WorldQuery},
    system::{
        lifetimeless::{SRes, SResMut},
        StaticSystemParam, SystemParam, SystemParamItem,
    },
};
use bevy_relationship::reexport::Mutable;
use imports::{QueryData, QueryItem};

use super::{NoTheme, ThemeComponent, ThemeDispatch};
use crate::{
    animation::{ease::AnimationEaseMethod, play_asset_animation, MaterialAnimationQueryData},
    prelude::*,
    util::set_component_or_insert,
};

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
                    let Ok(mut theme_component) = query.get_mut(event.target()) else {
                        return;
                    };

                    if theme_component.theme_entity != Entity::PLACEHOLDER {
                        return;
                    }
                    theme_component.theme_entity = theme_entity;
                }

                let mut query = widget_query.p0();
                let Ok(query_item) = query.get_mut(event.target()) else {
                    return;
                };

                let entity_commands = commands.entity(event.target());
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
            let Ok(theme_component) = query.get(event.target()) else {
                return;
            };
            theme_component.theme_entity
        };

        let Ok(this) = theme_query.get(theme_entity) else {
            return;
        };

        let mut query = widget_query.p1();
        let Ok(query_item) = query.get_mut(event.target()) else {
            return;
        };

        let entity_commands = commands.entity(event.target());
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

pub trait MaterialApplyMethod<M> {
    type ItemQuery: QueryData + 'static;
    type Params: SystemParam + 'static;

    fn apply(
        &self,
        material: M,
        commands: EntityCommands,
        query_items: QueryItem<Self::ItemQuery>,
        params: SystemParamItem<Self::Params>,
    );
}

#[derive(Default)]
pub struct SetMaterial;

impl<M: Component<Mutability = Mutable>> MaterialApplyMethod<M> for SetMaterial {
    type ItemQuery = Option<&'static mut M>;
    type Params = ();

    fn apply(
        &self,
        material: M,
        commands: EntityCommands,
        mut query_items: QueryItem<Self::ItemQuery>,
        _params: SystemParamItem<Self::Params>,
    ) {
        set_component_or_insert(query_items.as_deref_mut(), commands, material);
    }
}

#[derive(Clone, Debug)]
pub struct ApplyMaterialAnimation {
    pub duration: Duration,
    pub ease: AnimationEaseMethod,
}

impl<M: UiMaterial + Asset + Interpolation> MaterialApplyMethod<MaterialNode<M>>
    for ApplyMaterialAnimation
{
    type ItemQuery = MaterialAnimationQueryData<M>;
    type Params = SResMut<CallbackTypeRegister>;

    fn apply(
        &self,
        material: MaterialNode<M>,
        commands: EntityCommands,
        query_items: QueryItem<Self::ItemQuery>,
        mut callback_register: SystemParamItem<Self::Params>,
    ) {
        play_asset_animation(
            query_items,
            &mut callback_register,
            material.0,
            self.duration,
            self.ease.clone(),
            commands,
        );
    }
}

#[derive(Debug, Clone)]
pub struct InteractionMaterialSet<M: UiMaterial> {
    pub normal: MaterialNode<M>,
    pub hover: Option<MaterialNode<M>>,
    pub pressed: MaterialNode<M>,
}

impl<M: UiMaterial> Default for InteractionMaterialSet<M> {
    fn default() -> Self {
        Self {
            normal: Default::default(),
            hover: Default::default(),
            pressed: Default::default(),
        }
    }
}

impl<M: UiMaterial> InteractionMaterialSet<M> {
    pub fn new(assets: &mut Assets<M>, normal: M, hover: Option<M>, pressed: M) -> Self {
        Self {
            normal: assets.add(normal).into(),
            hover: hover.map(|hover| assets.add(hover).into()),
            pressed: assets.add(pressed).into(),
        }
    }

    pub fn get_material(&self, interaction: Interaction) -> &MaterialNode<M> {
        match interaction {
            Interaction::Pressed => &self.pressed,
            Interaction::Hovered => self.hover.as_ref().unwrap_or(&self.normal),
            Interaction::None => &self.normal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FocusMaterialSet<M: UiMaterial> {
    pub normal: MaterialNode<M>,
    pub focused: MaterialNode<M>,
}

impl<M: UiMaterial> Default for FocusMaterialSet<M> {
    fn default() -> Self {
        Self {
            normal: Default::default(),
            focused: Default::default(),
        }
    }
}

impl<M: UiMaterial> FocusMaterialSet<M> {
    pub fn new(assets: &mut Assets<M>, normal: M, focused: M) -> Self {
        Self {
            normal: assets.add(normal).into(),
            focused: assets.add(focused).into(),
        }
    }

    pub fn get_material(&self, focus: bool) -> &MaterialNode<M> {
        if focus {
            &self.focused
        } else {
            &self.normal
        }
    }
}
