use crate::prelude::*;

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum AnimationSystems {
    Finish,
}

#[derive(Component)]
pub struct DestroyAfterAnimationFinish;

pub fn after_animation_finish(
    mut events: EventReader<TweenCompleted>,
    animation_query: Query<Option<&DestroyAfterAnimationFinish>>,
    mut commands: Commands,
) {
    for event in events.read() {
        if let Ok(auto_destroy) = animation_query.get(event.entity) {
            if auto_destroy.is_some() {
                commands.entity(event.entity).despawn_recursive();
            }
        }
    }
}

pub fn despawn_animation<T: Component>(
    mut tween: Tween<T>,
) -> (Animator<T>, DestroyAfterAnimationFinish) {
    tween.set_completed_event(0);
    (Animator::new(tween), DestroyAfterAnimationFinish)
}

#[macro_export]
macro_rules! animation {
    ($time:literal secs:$func:ident->$t:ident($from:expr=>$to:expr)) => {
        Animator::new(animation!(Tween $time secs:$func->$t($from=>$to)))
    };
    (Tween $time:literal secs:$func:ident->$t:ident($from:expr=>$to:expr)) => {
        Tween::new(
            EaseFunction::$func,
            Duration::from_secs_f32($time),
            $t {
                start: $from,
                end: $to,
            },
        )
    };
}
