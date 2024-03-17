use crate::prelude::*;

use super::{AnimationEvent, AnimationState};

#[derive(Component, Debug, Clone)]
pub struct BackupStyle(pub Style);

pub fn with_backup_style<R>(
    event: &AnimationEvent,
    entity: &mut EntityWorldMut,
    f: impl FnOnce(&mut EntityWorldMut) -> R,
) -> R {
    if event.just_start {
        let style = entity.get::<Style>().unwrap();
        entity.insert(BackupStyle(style.clone()));
    }
    let r = f(entity);
    if event.just_finish {
        entity.remove::<BackupStyle>();
    }
    r
}

fn move_rect(style: &mut Style, offset: Vec2, size: Vec2) {
    match &mut style.top {
        Val::Px(ref mut v) => {
            *v += offset.y;
        }
        Val::Percent(ref mut v) => {
            *v += 100.0 * offset.y / size.y;
        }
        _ => {}
    }
    match &mut style.bottom {
        Val::Px(ref mut v) => {
            *v -= offset.y;
        }
        Val::Percent(ref mut v) => {
            *v -= 100.0 * offset.y / size.y;
        }
        _ => {}
    }
    match &mut style.left {
        Val::Px(ref mut v) => {
            *v += offset.x;
        }
        Val::Percent(ref mut v) => {
            *v += 100.0 * offset.x / size.x;
        }
        _ => {}
    }
    match &mut style.right {
        Val::Px(ref mut v) => {
            *v -= offset.x;
        }
        Val::Percent(ref mut v) => {
            *v -= 100.0 * offset.x / size.x;
        }
        _ => {}
    }
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

fn move_rect_by_percent(dest: &mut Style, src: &Style, offset: Vec2, size: Vec2) {
    move_val_by_percent(&mut dest.top, &src.top, offset.y, size.y);
    move_val_by_percent(&mut dest.bottom, &src.bottom, -offset.y, size.y);
    move_val_by_percent(&mut dest.left, &src.top, offset.x, size.x);
    move_val_by_percent(&mut dest.right, &src.bottom, -offset.x, size.x);
}

pub fn popup_open_drop_down(
    In(event @ AnimationEvent { entity, value, .. }): In<AnimationEvent>,
    world: &mut World,
) {
    with_backup_style(&event, &mut world.entity_mut(entity), |e| {
        let backup_style = e.get::<BackupStyle>().unwrap().clone();
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
        let backup_style = e.get::<BackupStyle>().unwrap().clone();
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
