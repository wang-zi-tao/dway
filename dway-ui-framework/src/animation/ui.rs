use interpolation::EaseFunction;

use super::{AnimationEaseMethod, AnimationEvent};
use crate::prelude::*;

#[derive(Component, Debug, Clone, Default)]
pub struct TargetStyle(pub Style);
impl From<Style> for TargetStyle {
    fn from(value: Style) -> Self {
        Self(value)
    }
}

pub fn with_backup_style<R>(
    event: &AnimationEvent,
    entity: &mut EntityWorldMut,
    f: impl FnOnce(&mut EntityWorldMut) -> R,
) -> R {
    if event.just_start {
        let style = entity.get::<Style>().unwrap();
        entity.insert(TargetStyle(style.clone()));
    }
    let r = f(entity);
    if event.just_finish {
        entity.remove::<TargetStyle>();
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

pub(crate) fn move_rect_by_percent(dest: &mut Style, src: &Style, offset: Vec2, size: Vec2) {
    move_val_by_percent(&mut dest.top, &src.top, offset.y, size.y);
    move_val_by_percent(&mut dest.bottom, &src.bottom, -offset.y, size.y);
    move_val_by_percent(&mut dest.left, &src.left, offset.x, size.x);
    move_val_by_percent(&mut dest.right, &src.bottom, -offset.x, size.x);
}

pub fn popup_open_drop_down(
    In(event @ AnimationEvent { entity, value, .. }): In<AnimationEvent>,
    world: &mut World,
) {
    with_backup_style(&event, &mut world.entity_mut(entity), |e| {
        let backup_style = e.get::<TargetStyle>().unwrap().clone();
        let size = e.get::<Node>().unwrap().size();
        move_rect_by_percent(
            &mut e.get_mut().unwrap(),
            &backup_style.0,
            Vec2::NEG_Y * (1.0 - value),
            size,
        );
    });
}

pub fn popup_open_close_up(
    In(event @ AnimationEvent { entity, value, .. }): In<AnimationEvent>,
    world: &mut World,
) {
    with_backup_style(&event, &mut world.entity_mut(entity), |e| {
        let backup_style = e.get::<TargetStyle>().unwrap().clone();
        let size = e.get::<Node>().unwrap().size();
        move_rect_by_percent(
            &mut e.get_mut().unwrap(),
            &backup_style.0,
            Vec2::NEG_Y * value,
            size,
        );
    });
    if event.just_finish {
        world.entity_mut(entity).despawn_recursive();
    }
}

pub fn despawn_recursive_on_animation_finish(
    In(event): In<AnimationEvent>,
    mut commands: Commands,
) {
    if event.just_finish {
        commands.entity(event.entity).despawn_recursive();
    }
}

pub trait UiAnimationConfig {
    fn appear_time() -> Duration {
        Duration::from_secs_f32(0.5)
    }
    fn appear_ease() -> AnimationEaseMethod {
        EaseFunction::QuarticIn.into()
    }
    fn appear_animation(theme: &Theme) -> SystemId<AnimationEvent>;
    fn disappear_time() -> Duration {
        Duration::from_secs_f32(0.5)
    }
    fn disappear_ease() -> AnimationEaseMethod {
        EaseFunction::QuarticOut.into()
    }
    fn disappear_animation(theme: &Theme) -> SystemId<AnimationEvent>;
}

pub struct UiAnimationDropdownConfig;
impl UiAnimationConfig for UiAnimationDropdownConfig {
    fn appear_animation(theme: &Theme) -> SystemId<AnimationEvent> {
        theme.system(popup_open_drop_down)
    }

    fn disappear_animation(theme: &Theme) -> SystemId<AnimationEvent> {
        theme.system(popup_open_close_up)
    }
}
