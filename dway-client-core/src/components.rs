use bevy::prelude::*;
use smallvec::SmallVec;
use std::ops::Deref;

#[derive(Component, Default)]
pub struct OutputMark;

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AttachToOutput(pub SmallVec<[Entity; 1]>);
