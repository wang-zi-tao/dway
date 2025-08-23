use std::marker::PhantomData;

use bevy::ecs::entity::EntityHashMap;
use imports::{QueryData, QueryFilter};

use crate::prelude::*;

pub struct SubWidgetEntity {
    pub widget: Entity,
    pub sub_widget: Entity,
}

#[derive(Resource)]
pub struct WidgetQueryState<B: Bundle, Q: QueryData, F: QueryFilter> {
    widget_data_map: EntityHashMap<Vec<SubWidgetEntity>>,
    phantom: PhantomData<(B, Q, F)>,
}

impl<B: Bundle, Q: QueryData, F: QueryFilter> WidgetQueryState<B, Q, F> {
    pub fn update(query: Query<(Entity, Q), F>) {
    }

    pub fn check_entity(entity: Entity, query: Query<(Entity, Q), F>) {
        if let Ok((entity, data)) = query.get(entity) {}
    }

    pub fn on_insert(trigger: Trigger<OnInsert, B>, query: Query<(Entity, Q), F>) {
        Self::check_entity(trigger.target(), query);
    }

    pub fn on_remove(trigger: Trigger<OnRemove, B>, query: Query<(Entity, Q), F>) {
        Self::check_entity(trigger.target(), query);
    }
}
