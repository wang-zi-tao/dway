use std::{fmt::Debug, sync::Arc};

use bevy::{app::DynEq, ecs::{component::HookContext, label::DynHash}, platform::collections::HashMap};
use bevy_relationship::reexport::{Mutable, SmallVec, StorageType};

use crate::prelude::*;

pub trait AnimationKeyTrait: DynHash + DynEq + Debug + Send + Sync + 'static {
    fn get_dyn_eq(&self) -> &dyn DynEq;
}
impl<T: DynHash + DynEq + Debug + Send + Sync + 'static> AnimationKeyTrait for T {
    fn get_dyn_eq(&self) -> &dyn DynEq {
        self
    }
}

#[derive(Debug, Clone)]
pub enum AnimationKey {
    Entity(Entity),
    String(String),
    Other(Arc<dyn AnimationKeyTrait>),
}

impl PartialEq for AnimationKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Entity(l0), Self::Entity(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Other(l0), Self::Other(r0)) => l0.dyn_eq(r0.get_dyn_eq()),
            _ => false,
        }
    }
}
impl Eq for AnimationKey {
}

impl std::hash::Hash for AnimationKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            AnimationKey::Entity(e) => e.hash(state),
            AnimationKey::String(e) => e.hash(state),
            AnimationKey::Other(e) => e.dyn_hash(state),
        }
    }
}

impl Component for AnimationKey {
    type Mutability = Mutable;
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut bevy::ecs::component::ComponentHooks) {
        hooks.on_insert(|mut world, context: HookContext| {
            let key = world.entity(context.entity).get::<Self>().unwrap().clone();
            let mut register = world.resource_mut::<AnimationRegister>();
            register.add(context.entity, key);
        });
        hooks.on_remove(|mut world, context: HookContext| {
            let key = world.entity(context.entity).get::<Self>().unwrap().clone();
            let mut register = world.resource_mut::<AnimationRegister>();
            register.remove(context.entity, &key);
        });
    }
}

#[derive(Resource, Default, Debug)]
pub struct AnimationRegister {
    keys_map: HashMap<AnimationKey, SmallVec<[Entity; 1]>>,
}

impl AnimationRegister {
    fn add(&mut self, entity: Entity, key: AnimationKey) {
        self.keys_map.entry(key).or_default().push(entity);
    }

    fn remove(&mut self, entity: Entity, key: &AnimationKey) {
        if let Some(entitys) = self.keys_map.get_mut(key) {
            if let Some(index) = entitys.as_slice().iter().position(|x| x == &entity) {
                entitys.swap_remove(index);
            }
        }
    }
}
