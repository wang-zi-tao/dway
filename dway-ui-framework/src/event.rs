use bevy::ecs::system::EntityCommands;

use crate::prelude::*;

#[bevy_trait_query::queryable]
pub trait EventDispatch<E> {
    fn on_event(&self, commands: EntityCommands, event: E);
}

#[derive(Clone, Debug)]
pub struct UiClickEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiNodeAppearEvent {
    Appear,
    Disappear,
}

impl UiNodeAppearEvent {
    pub fn appear(&self) -> bool {
        match self {
            UiNodeAppearEvent::Appear => true,
            UiNodeAppearEvent::Disappear => false,
        }
    }
}
