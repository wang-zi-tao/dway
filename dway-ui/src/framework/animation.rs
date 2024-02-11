use bevy::window::RequestRedraw;
// use bevy_tweening::{AnimatorState, AssetAnimator};

use crate::prelude::*;

#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum AnimationSystems {
    Finish,
    PrepareNextFrame,
}

#[derive(Component)]
pub struct DestroyAfterAnimationFinish;

pub fn after_animation_finish(
    // mut events: EventReader<TweenCompleted>,
    animation_query: Query<Option<&DestroyAfterAnimationFinish>>,
    mut commands: Commands,
) {
    // for event in events.read() {
    //     if let Ok(auto_destroy) = animation_query.get(event.entity) {
    //         if auto_destroy.is_some() {
    //             commands.entity(event.entity).despawn_recursive();
    //         }
    //     }
    // }
}

// pub fn request_update_system(
//     style_animator_query: Query<&Animator<Style>, With<Animator<Style>>>,
//     transform_animator_query: Query<&Animator<Transform>, With<Animator<Transform>>>,
//     rounded_rect_animator_query: Query<
//         &AssetAnimator<RoundedUiRectMaterial>,
//         With<AssetAnimator<RoundedUiRectMaterial>>,
//     >,
//     circle_animator_query: Query<
//         &AssetAnimator<UiCircleMaterial>,
//         With<AssetAnimator<UiCircleMaterial>>,
//     >,
//     mut event_sender: EventWriter<RequestRedraw>,
// ) {
//     let mut animation_playing = false;
//     style_animator_query.for_each(|e| {
//         animation_playing |= e.tweenable().progress() < 1.0;
//     });
//     transform_animator_query.for_each(|e| {
//         animation_playing |= e.tweenable().progress() < 1.0;
//     });
//     rounded_rect_animator_query.for_each(|e| {
//         animation_playing |= e.tweenable().progress() < 1.0;
//     });
//     circle_animator_query.for_each(|e| {
//         animation_playing |= e.tweenable().progress() < 1.0;
//     });
//     if animation_playing {
//         event_sender.send(RequestRedraw);
//     }
// }
//
// pub fn despawn_animation<T: Component>(
//     mut tween: Tween<T>,
// ) -> (Animator<T>, DestroyAfterAnimationFinish) {
//     tween.set_completed_event(0);
//     (Animator::new(tween), DestroyAfterAnimationFinish)
// }
//
// #[macro_export]
// macro_rules! animation {
//     ($time:literal secs:$func:ident->$t:ident($from:expr=>$to:expr)) => {
//         Animator::new(animation!(Tween $time secs:$func->$t($from=>$to)))
//     };
//     (Tween $time:literal secs:$func:ident->$t:ident($from:expr=>$to:expr)) => {
//         Tween::new(
//             EaseFunction::$func,
//             Duration::from_secs_f32($time),
//             $t {
//                 start: $from,
//                 end: $to,
//             },
//         )
//     };
// }
