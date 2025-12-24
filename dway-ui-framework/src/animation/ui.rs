use interpolation::EaseFunction;

use super::{AnimationEaseMethod, AnimationEvent};
use crate::{
    event::{CallbackTypeRegister, UiEvent},
    prelude::*,
};

#[derive(Component, Debug, Clone, Default, Reflect)]
pub struct AnimationTargetNodeState(pub Node);
impl From<Node> for AnimationTargetNodeState {
    fn from(value: Node) -> Self {
        Self(value)
    }
}

pub fn with_backup_style<R>(
    event: &AnimationEvent,
    entity: &mut EntityWorldMut,
    f: impl FnOnce(&mut EntityWorldMut) -> R,
) -> R {
    if event.just_start {
        let style = entity.get::<Node>().unwrap();
        entity.insert(AnimationTargetNodeState(style.clone()));
    }
    let r = f(entity);
    if event.just_finish {
        entity.remove::<AnimationTargetNodeState>();
    }
    r
}

fn move_val_by_percent(dest: &mut Val, src: &Val, offset: f32, size: f32) {
    match (dest, src) {
        (Val::Px(d), Val::Px(s)) => {
            *d = *s + offset * size;
        }
        (Val::Percent(d), Val::Percent(s)) => {
            *d = *s + 100.0 * offset;
        }
        _ => {}
    }
}

pub(crate) fn move_rect_by_percent(dest: &mut Node, src: &Node, offset: Vec2, size: Vec2) {
    move_val_by_percent(&mut dest.top, &src.top, offset.y, size.y);
    move_val_by_percent(&mut dest.bottom, &src.bottom, -offset.y, size.y);
    move_val_by_percent(&mut dest.left, &src.left, offset.x, size.x);
    move_val_by_percent(&mut dest.right, &src.bottom, -offset.x, size.x);
}

pub fn popup_open_drop_down(event: UiEvent<AnimationEvent>, world: &mut World) {
    let Ok(mut entity_mut) = world.get_entity_mut(event.receiver()) else {
        return;
    };
    with_backup_style(&event, &mut entity_mut, |e| {
        let backup_style = e.get::<AnimationTargetNodeState>().unwrap().clone();
        let size = e.get::<ComputedNode>().unwrap().size();
        move_rect_by_percent(
            &mut e.get_mut().unwrap(),
            &backup_style.0,
            Vec2::NEG_Y * (1.0 - event.value),
            size,
        );
    });
}

pub fn popup_open_close_up(event: UiEvent<AnimationEvent>, world: &mut World) {
    let Ok(mut entity_mut) = world.get_entity_mut(event.receiver()) else {
        return;
    };
    with_backup_style(&event, &mut entity_mut, |e| {
        let backup_style = e.get::<AnimationTargetNodeState>().unwrap().clone();
        let size = e.get::<ComputedNode>().unwrap().size();
        move_rect_by_percent(
            &mut e.get_mut().unwrap(),
            &backup_style.0,
            Vec2::NEG_Y * event.value,
            size,
        );
    });
    if event.just_finish {
        entity_mut.despawn();
    }
}

pub fn despawn_on_animation_finish(
    event: UiEvent<AnimationEvent>,
    mut commands: Commands,
) {
    if event.just_finish {
        commands.entity(event.receiver()).despawn();
    }
}

pub trait UiAnimationConfig {
    fn appear_time() -> Duration {
        Duration::from_secs_f32(0.5)
    }
    fn appear_ease() -> AnimationEaseMethod {
        EaseFunction::QuarticIn.into()
    }
    fn appear_animation(callbacks: &CallbackTypeRegister) -> SystemId<UiEvent<AnimationEvent>>;
    fn disappear_time() -> Duration {
        Duration::from_secs_f32(0.5)
    }
    fn disappear_ease() -> AnimationEaseMethod {
        EaseFunction::QuarticOut.into()
    }
    fn disappear_animation(callbacks: &CallbackTypeRegister) -> SystemId<UiEvent<AnimationEvent>>;
}

pub struct UiAnimationDropdownConfig;
impl UiAnimationConfig for UiAnimationDropdownConfig {
    fn appear_animation(callbacks: &CallbackTypeRegister) -> SystemId<UiEvent<AnimationEvent>> {
        callbacks.system(popup_open_drop_down)
    }

    fn disappear_animation(callbacks: &CallbackTypeRegister) -> SystemId<UiEvent<AnimationEvent>> {
        callbacks.system(popup_open_close_up)
    }
}
