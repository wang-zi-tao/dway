use std::sync::Arc;

use interpolation::{Ease, EaseFunction};

use super::AnimationDirection;
use crate::prelude::*;

#[derive(Clone)]
pub enum AnimationEaseMethod {
    EaseFunction(EaseFunction),
    Linear,
    Step(f32),
    Lambda(Arc<dyn Fn(f32) -> f32 + Send + Sync + 'static>),
}

impl AnimationEaseMethod {
    pub fn set_direction(&mut self, direction: AnimationDirection) -> bool {
        match self {
            AnimationEaseMethod::EaseFunction(e) => {
                let select_ease = |neg: EaseFunction, pos: EaseFunction| match direction {
                    AnimationDirection::Positive => pos,
                    AnimationDirection::Negative => neg,
                };
                use EaseFunction::*;
                *e = match e {
                    QuadraticIn | QuadraticOut => select_ease(QuadraticIn, QuadraticOut),
                    QuadraticInOut => QuadraticInOut,
                    CubicIn | CubicOut => select_ease(CubicIn, CubicOut),
                    CubicInOut => CubicInOut,
                    QuarticIn | QuarticOut => select_ease(QuarticIn, QuarticOut),
                    QuarticInOut => QuarticInOut,
                    QuinticIn | QuinticOut => select_ease(QuinticIn, QuinticOut),
                    QuinticInOut => QuinticInOut,
                    SineIn | SineOut => select_ease(SineIn, SineOut),
                    SineInOut => SineInOut,
                    CircularIn | CircularOut => select_ease(CircularIn, CircularOut),
                    CircularInOut => CircularInOut,
                    ExponentialIn | ExponentialOut => select_ease(ExponentialIn, ExponentialOut),
                    ExponentialInOut => ExponentialInOut,
                    ElasticIn | ElasticOut => select_ease(ElasticIn, ElasticOut),
                    ElasticInOut => ElasticInOut,
                    BackIn | BackOut => select_ease(BackIn, BackOut),
                    BackInOut => BackInOut,
                    BounceIn | BounceOut => select_ease(BounceIn, BounceOut),
                    BounceInOut => BounceInOut,
                };
                true
            }
            AnimationEaseMethod::Linear => true,
            AnimationEaseMethod::Step(_) => false,
            AnimationEaseMethod::Lambda(_) => false,
        }
    }
}

impl From<EaseFunction> for AnimationEaseMethod {
    fn from(value: EaseFunction) -> Self {
        AnimationEaseMethod::EaseFunction(value)
    }
}

impl Default for AnimationEaseMethod {
    fn default() -> Self {
        Self::EaseFunction(EaseFunction::CubicIn)
    }
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
