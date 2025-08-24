mod asset;
pub mod ease;
pub mod registry;
pub mod translation;
pub mod ui;

use std::marker::PhantomData;

pub use asset::*;
use bevy::window::RequestRedraw;
use ease::AnimationEaseMethod;
pub use interpolation;
use interpolation::{Ease, EaseFunction};
use registry::AnimationRegister;

use crate::{
    command::DestroyInterceptor,
    event::{
        CallbackRegisterAppExt, CallbackTypeRegister, EventDispatcher, EventReceiver, UiEvent,
        UiNodeAppearEvent,
    },
    prelude::*,
    UiFrameworkSystems,
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
        LinearRgba::from_f32_array(Interpolation::interpolation(
            &self.to_linear().to_f32_array(),
            &other.to_linear().to_f32_array(),
            v,
        ))
        .into()
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

impl Interpolation for LinearRgba {
    fn interpolation(&self, other: &Self, v: f32) -> Self {
        Self {
            red: Interpolation::interpolation(&self.red, &other.red, v),
            green: Interpolation::interpolation(&self.green, &other.green, v),
            blue: Interpolation::interpolation(&self.blue, &other.blue, v),
            alpha: Interpolation::interpolation(&self.alpha, &other.alpha, v),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationEvent {
    pub value: f32,
    pub old_value: f32,
    pub just_start: bool,
    pub just_finish: bool,
}

pub type AnimationEventDispatcher = EventDispatcher<AnimationEvent>;

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
            direction: AnimationDirection::Positive,
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
    mut query: Query<(Entity, &mut Animation, &EventDispatcher<AnimationEvent>)>,
    time: Res<Time>,
    mut redraw_request: EventWriter<RequestRedraw>,
    mut commands: Commands,
) {
    let mut play = false;
    for (entity, mut animation, event_dispatcher) in &mut query {
        if animation.state != AnimationState::Play {
            continue;
        }
        play = true;
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

        event_dispatcher.send(
            AnimationEvent {
                value: ease,
                old_value: ease_old,
                just_start: animation.clock.duration == Duration::ZERO,
                just_finish: duration > animation.clock.total_duration,
            },
            &mut commands,
        );

        if duration > animation.clock.total_duration {
            animation.state = AnimationState::Finished;
        }
        animation.clock.duration = duration;
    }
    if play {
        redraw_request.send(RequestRedraw);
    }
}

#[derive(Clone, Default, Component)]
pub struct Tween<I: UiMaterial + Interpolation + Asset> {
    pub begin: Handle<I>,
    pub end: Handle<I>,
}

impl<I: UiMaterial + Interpolation + Asset> Tween<I> {
    pub fn new(begin: Handle<I>, end: Handle<I>) -> Self {
        Self { begin, end }
    }

    pub fn reverse(&mut self) {
        let Self { begin, end } = self;
        std::mem::swap(begin, end);
    }
}

pub struct AnimationPlugin;
impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            update_animation_system
                .in_set(UiFrameworkSystems::ApplyAnimation),
        )
        .init_resource::<AnimationRegister>()
        .register_component_as::<dyn EventReceiver<AnimationEvent>, translation::UiTranslationAnimation>()
        .register_component_as::<dyn EventReceiver<UiNodeAppearEvent>, translation::UiTranslationAnimation>()
        .register_component_as::<dyn EventReceiver<UiPopupEvent>, translation::UiTranslationAnimation>()
        .register_component_as::<dyn DestroyInterceptor, translation::UiTranslationAnimation>()
        .register_callback(ui::popup_open_drop_down)
        .register_callback(ui::popup_open_close_up)
        .register_callback(ui::despawn_recursive_on_animation_finish);
    }
}
