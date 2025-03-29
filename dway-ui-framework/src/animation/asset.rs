use std::marker::PhantomData;

use bevy::ecs::query::{QueryData, QueryItem};

use super::{ease::AnimationEaseMethod, AnimationEvent, AnimationEventDispatcher};
use crate::{prelude::*, util::modify_component_or_insert};

#[derive(QueryData)]
#[query_data(mutable)]
pub struct MaterialAnimationQueryData<M: UiMaterial + Asset + Interpolation> {
    ui_material: Option<&'static mut MaterialNode<M>>,
    tween: Option<&'static mut Tween<M>>,
    animation: Option<&'static mut Animation>,
    animation_event: Option<&'static mut AnimationEventDispatcher>,
}

pub fn play_asset_animation<M: UiMaterial + Asset + Interpolation>(
    query_item: QueryItem<MaterialAnimationQueryData<M>>,
    callback_register: &mut CallbackTypeRegister,
    end_material: Handle<M>,
    duration: Duration,
    ease: AnimationEaseMethod,
    mut entity_commands: EntityCommands,
) {
    let MaterialAnimationQueryDataItem {
        ui_material,
        tween,
        mut animation,
        mut animation_event,
    } = query_item;

    let current_material = ui_material;
    if let Some(current_material) = current_material {
        let new_tween = Tween::new(current_material.0.clone(), end_material);
        if let Some(mut tween) = tween {
            *tween = new_tween;
        } else {
            let system = callback_register.system(apply_tween_asset::<M>);
            modify_component_or_insert(animation_event.as_deref_mut(), entity_commands.reborrow(), move |c| {
                c.add_system_to_this(system);
            });

            entity_commands.insert(new_tween);
        }

        modify_component_or_insert(animation.as_deref_mut(), entity_commands.reborrow(), move |a| {
            a.set_duration(duration);
            a.set_ease_method(ease);
            a.replay();
        });
    } else {
        entity_commands.insert(MaterialNode(end_material));
    }
}

pub fn apply_tween_asset<I: UiMaterial + Interpolation + Asset>(
    event: UiEvent<AnimationEvent>,
    mut assets: ResMut<Assets<I>>,
    mut query: Query<(&mut MaterialNode<I>, &Tween<I>)>,
) {
    let entity = event.receiver();
    let AnimationEvent { value, .. } = *event;
    let Ok((mut material, tween)) = query.get_mut(entity) else {
        return;
    };
    if value <= 0.0 {
        material.0 = tween.begin.clone();
    } else if value >= 1.0 {
        material.0 = tween.end.clone();
    } else if let (Some(begin_asset), Some(end_asset)) =
        (assets.get(&tween.begin), assets.get(&tween.end))
    {
        let new_asset = Interpolation::interpolation(begin_asset, end_asset, value);
        if material.0 == tween.begin || material.0 == tween.end {
            material.0 = assets.add(new_asset);
        } else {
            assets.insert(material.id(), new_asset);
        }
    }
}

#[derive(Default)]
pub struct AssetAnimationPlugin<T: UiMaterial + Interpolation + Asset>(PhantomData<T>);

impl<T: UiMaterial + Interpolation + Asset> Plugin for AssetAnimationPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_callback(apply_tween_asset::<T>);
    }

    fn is_unique(&self) -> bool {
        false
    }
}

#[derive(Bundle)]
pub struct AssetTweenExt<T: UiMaterial + Interpolation + Asset> {
    pub animation: Animation,
    pub event_dispatcher: EventDispatcher<AnimationEvent>,
    pub tween: Tween<T>,
}

impl<T: UiMaterial + Interpolation + Asset> AssetTweenExt<T> {
    pub fn new(animation: Animation, tween: Tween<T>, callbacks: &CallbackTypeRegister) -> Self {
        let event_dispatcher = EventDispatcher::default()
            .with_system_to_this(callbacks.system(apply_tween_asset::<T>));
        Self {
            animation,
            tween,
            event_dispatcher,
        }
    }
}
