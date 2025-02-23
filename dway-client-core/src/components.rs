use bevy::prelude::*;
use smallvec::SmallVec;

#[derive(Component, Default)]
pub struct OutputMark;

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AttachToOutput(pub SmallVec<[Entity; 1]>);
