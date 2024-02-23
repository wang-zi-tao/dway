use crate::{prelude::*, theme::ThemeAppExt};
use bevy::ecs::system::Command;
use bevy_relationship::reexport::SmallVec;
pub use interpolation;
use interpolation::{Ease, EaseFunction};
use std::{marker::PhantomData, sync::Arc};

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
        Color::rgba_from_array(Interpolation::interpolation(&self.rgba_to_vec4(),&other.rgba_to_vec4(),v))
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

#[derive(Clone)]
pub enum AnimationEaseMethod {
    EaseFunction(EaseFunction),
    Linear,
    Step(f32),
    Lambda(Arc<dyn Fn(f32) -> f32 + Send + Sync + 'static>),
}
impl AnimationEaseMethod {
    pub fn calc(&self, value: f32) -> f32 {
        match self {
            AnimationEaseMethod::EaseFunction(f) => value.calc(*f),
            AnimationEaseMethod::Lambda(f) => f(value),
            AnimationEaseMethod::Linear => value,
            AnimationEaseMethod::Step(c) => value - value % c,
        }
    }
}

impl From<EaseFunction> for AnimationEaseMethod {
    fn from(value: EaseFunction) -> Self {
        Self::EaseFunction(value)
    }
}

impl std::fmt::Debug for AnimationEaseMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EaseFunction(arg0) => f.debug_tuple("EaseFunction").field(arg0).finish(),
            Self::Lambda(_) => f.debug_tuple("Lambda").finish(),
            AnimationEaseMethod::Linear => f.debug_tuple("Linear").finish(),
            AnimationEaseMethod::Step(c) => f.debug_tuple("Step").field(&c).finish(),
        }
    }
}

structstruck::strike! {
    #[derive(Component)]
    #[strikethrough[derive(Debug, Clone)]]
    pub struct Animation {
        pub state:
        #[derive(PartialEq, Eq)]
        enum AnimationState{
            Play,
            Pause,
            Finished,
        },
        pub ease: AnimationEaseMethod,
        pub clock: struct AnimationClock {
            duration: Duration,
            total_duration: Duration,
        },
        pub callbacks: SmallVec<[SystemId<(Entity,f32)>; 2]>,
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
        }
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
    pub fn set_duration(&mut self, duration: Duration) {
        self.clock.total_duration = duration;
    }
    pub fn set_ease_method(&mut self, ease: AnimationEaseMethod) {
        self.ease = ease;
    }
}

pub fn update_animation_system(
    mut query: Query<(Entity, &mut Animation)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut animation) in &mut query {
        if animation.state != AnimationState::Play {
            continue;
        }
        let duration = animation.clock.duration + time.delta();
        let ease = animation
            .ease
            .calc(duration.as_secs_f32() / animation.clock.total_duration.as_secs_f32());
        for callback in &animation.callbacks {
            commands.run_system_with_input(*callback, (entity, ease));
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
    In((entity, v)): In<(Entity, f32)>,
    mut assets: ResMut<Assets<I>>,
    mut query: Query<(&mut Handle<I>, &Tween<I>)>,
) {
    let Ok((mut handle, tween)) = query.get_mut(entity) else {
        return;
    };
    if v <= 0.0 {
        *handle = tween.begin.clone();
    } else if v >= 1.0 {
        *handle = tween.end.clone();
    } else if let (Some(begin_asset), Some(end_asset)) =
        (assets.get(&tween.begin), assets.get(&tween.end))
    {
        let new_asset = Interpolation::interpolation(begin_asset, end_asset, v);
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
pub struct AssetTweenAddonBundle<T: Interpolation + Asset> {
    animation: Animation,
    tween: Tween<T>,
}

impl<T: Interpolation + Asset> AssetTweenAddonBundle<T> {
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
            update_animation_system.in_set(UiFrameworkSystems::ApplyAnimation),
        );
    }
}
