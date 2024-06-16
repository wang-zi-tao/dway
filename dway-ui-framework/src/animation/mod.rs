pub mod ease;
pub mod translation;
pub mod ui;

use std::{marker::PhantomData, sync::Arc};

use bevy::ecs::system::EntityCommands;
use bevy_relationship::reexport::SmallVec;
use ease::AnimationEaseMethod;
pub use interpolation;
use interpolation::{Ease, EaseFunction};

use crate::{
    event::{EventDispatch, UiNodeAppearEvent},
    prelude::*,
};

pub trait Interpolation {
    fn interpolation(&self, other: &Self, v: f32) -> Self;
}

macro_rules! make_interpolation {
    ($t:ty) => {
        impl Interpolation for $t {
            fn interpolation(&self, other: &Self, v: f32) -> Self {
                *self * (1.0 - v) + *other * v
            }
        }
    };
}

make_interpolation!(f32);
make_interpolation!(Vec2);
make_interpolation!(Vec3);
make_interpolation!(Vec4);
impl Interpolation for Color {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        Color::rgba_from_array(Interpolation::interpolation(
            &self.rgba_to_vec4(),
            &other.rgba_to_vec4(),
            v,
        ))
    }
}

impl<T: Interpolation> Interpolation for [T; 1] {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        [Interpolation::interpolation(&self[0], &other[0], v)]
    }
}
impl<T: Interpolation> Interpolation for [T; 2] {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        [
            Interpolation::interpolation(&self[0], &other[0], v),
            Interpolation::interpolation(&self[1], &other[1], v),
        ]
    }
}
impl<T: Interpolation> Interpolation for [T; 3] {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        [
            Interpolation::interpolation(&self[0], &other[0], v),
            Interpolation::interpolation(&self[1], &other[1], v),
            Interpolation::interpolation(&self[2], &other[2], v),
        ]
    }
}
impl<T: Interpolation> Interpolation for [T; 4] {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        [
            Interpolation::interpolation(&self[0], &other[0], v),
            Interpolation::interpolation(&self[1], &other[1], v),
            Interpolation::interpolation(&self[2], &other[2], v),
            Interpolation::interpolation(&self[3], &other[3], v),
        ]
    }
}

#[derive(Clone, Debug)]
pub struct AnimationEvent {
    pub entity: Entity,
    pub value: f32,
    pub old_value: f32,
    pub just_start: bool,
    pub just_finish: bool,
}

structstruck::strike! {
    #[derive(Component)]
    #[strikethrough[derive(Debug, Clone, Reflect)]]
    pub struct Animation {
        pub state:
        #[derive(PartialEq, Eq)]
        enum AnimationState{
            Play,
            Pause,
            Finished,
        },
        pub direction:
        #[derive(PartialEq, Eq, Default, Copy)]
        enum AnimationDirection{
            #[default]
            Positive,
            Negative,
        },
        #[reflect(ignore)]
        pub ease: AnimationEaseMethod,
        pub clock: struct AnimationClock {
            duration: Duration,
            total_duration: Duration,
        },
        #[reflect(ignore)]
        pub callbacks: SmallVec<[SystemId<AnimationEvent>; 2]>,
    }
}

impl AnimationDirection {
    pub fn new(value: bool) -> Self {
        if value {
            Self::Positive
        } else {
            Self::Negative
        }
    }
}

impl Default for Animation {
    fn default() -> Self {
        Animation::new(Duration::from_secs_f32(0.5), EaseFunction::CubicOut)
    }
}

impl Animation {
    pub fn new(duration: Duration, ease: impl Into<AnimationEaseMethod>) -> Self {
        Self {
            state: AnimationState::Play,
            ease: ease.into(),
            clock: AnimationClock {
                duration: Duration::ZERO,
                total_duration: duration,
            },
            callbacks: Default::default(),
            direction: AnimationDirection::Positive,
        }
    }

    pub fn with_callback(mut self, callback: SystemId<AnimationEvent>) -> Self {
        self.callbacks.push(callback);
        self
    }

    pub fn pause(&mut self) {
        if self.state != AnimationState::Play {
            self.state = AnimationState::Pause;
        }
    }

    pub fn replay(&mut self) {
        match self.state {
            AnimationState::Play => {
                self.clock.duration = Duration::ZERO;
            }
            AnimationState::Pause => {
                self.state = AnimationState::Play;
            }
            AnimationState::Finished => {
                self.clock.duration = Duration::ZERO;
                self.state = AnimationState::Play;
            }
        }
        self.state = AnimationState::Play;
    }

    pub fn play(&mut self) {
        match self.state {
            AnimationState::Play => {}
            AnimationState::Pause => {
                self.state = AnimationState::Play;
            }
            AnimationState::Finished => {
                self.clock.duration = Duration::ZERO;
                self.state = AnimationState::Play;
            }
        }
        self.state = AnimationState::Play;
    }

    pub fn play_with_direction(&mut self, direction: AnimationDirection) {
        match self.state {
            AnimationState::Play | AnimationState::Pause => {
                if direction != self.direction {
                    self.clock.duration = self.clock.total_duration - self.clock.duration;
                    self.ease.set_direction(direction);
                }
            }
            AnimationState::Finished => {
                self.clock.duration = Duration::ZERO;
                self.state = AnimationState::Play;
            }
        }
        self.direction = direction;
        self.state = AnimationState::Play;
    }

    pub fn set_duration(&mut self, duration: Duration) {
        self.clock.total_duration = duration;
    }

    pub fn set_ease_method(&mut self, ease: AnimationEaseMethod) {
        self.ease = ease;
    }
}

pub fn update_animation_system(
    mut query: Query<(Entity, &mut Animation, &dyn EventDispatch<AnimationEvent>)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut animation, dispatchs) in &mut query {
        if animation.state != AnimationState::Play {
            continue;
        }
        let duration = animation.clock.duration + time.delta();
        let mut ease_old = animation.ease.calc(
            animation.clock.duration.as_secs_f32() / animation.clock.total_duration.as_secs_f32(),
        );
        let mut ease = animation
            .ease
            .calc(duration.as_secs_f32() / animation.clock.total_duration.as_secs_f32());
        if animation.clock.duration == Duration::ZERO {
            ease_old = 0.0;
        }
        if duration > animation.clock.total_duration {
            ease = 1.0;
        }
        {
            let animation_event = AnimationEvent {
                entity,
                value: ease,
                old_value: ease_old,
                just_start: animation.clock.duration == Duration::ZERO,
                just_finish: duration > animation.clock.total_duration,
            };
            for callback in &animation.callbacks {
                commands.run_system_with_input(*callback, animation_event.clone());
            }
            for dispatch in &dispatchs {
                let mut entity_commands = commands.entity(entity);
                dispatch.on_event(entity_commands.reborrow(), animation_event.clone());
            }
        }

        if duration > animation.clock.total_duration {
            animation.state = AnimationState::Finished;
        }
        animation.clock.duration = duration;
    }
}

#[derive(Clone, Default, Component)]
pub struct Tween<I: Interpolation + Asset> {
    pub begin: Handle<I>,
    pub end: Handle<I>,
}

impl<I: Interpolation + Asset> Tween<I> {
    pub fn new(begin: Handle<I>, end: Handle<I>) -> Self {
        Self { begin, end }
    }

    pub fn reverse(&mut self) {
        let Self { begin, end } = self;
        std::mem::swap(begin, end);
    }
}

pub fn apply_tween_asset<I: Interpolation + Asset>(
    In(AnimationEvent { entity, value, .. }): In<AnimationEvent>,
    mut assets: ResMut<Assets<I>>,
    mut query: Query<(&mut Handle<I>, &Tween<I>)>,
) {
    let Ok((mut handle, tween)) = query.get_mut(entity) else {
        return;
    };
    if value <= 0.0 {
        *handle = tween.begin.clone();
    } else if value >= 1.0 {
        *handle = tween.end.clone();
    } else if let (Some(begin_asset), Some(end_asset)) =
        (assets.get(&tween.begin), assets.get(&tween.end))
    {
        let new_asset = Interpolation::interpolation(begin_asset, end_asset, value);
        if &*handle == &tween.begin || &*handle == &tween.end {
            *handle = assets.add(new_asset);
        } else {
            assets.insert(handle.clone(), new_asset);
        }
    }
}

#[derive(Default)]
pub struct AssetAnimationPlugin<T: Interpolation + Asset>(PhantomData<T>);

impl<T: Interpolation + Asset> Plugin for AssetAnimationPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_system(apply_tween_asset::<T>);
    }

    fn is_unique(&self) -> bool {
        false
    }
}

#[derive(Bundle)]
pub struct AssetTweenExt<T: Interpolation + Asset> {
    animation: Animation,
    tween: Tween<T>,
}

impl<T: Interpolation + Asset> AssetTweenExt<T> {
    pub fn new(mut animation: Animation, tween: Tween<T>, theme: &Theme) -> Self {
        animation
            .callbacks
            .push(theme.system(apply_tween_asset::<T>));
        Self { animation, tween }
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (update_animation_system, apply_deferred)
                .chain()
                .in_set(UiFrameworkSystems::ApplyAnimation),
        )
        .register_component_as::<dyn EventDispatch<AnimationEvent>, translation::UiTranslationAnimation>()
        .register_component_as::<dyn EventDispatch<UiNodeAppearEvent>, translation::UiTranslationAnimation>()
        .register_component_as::<dyn EventDispatch<PopupEvent>, translation::UiTranslationAnimation>()
        .register_system(ui::popup_open_drop_down)
        .register_system(ui::popup_open_close_up)
        .register_system(ui::despawn_recursive_on_animation_finish);
    }
}
