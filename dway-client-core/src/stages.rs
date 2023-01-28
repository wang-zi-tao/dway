use bevy::prelude::{Stage, StageLabel};


#[derive(Hash,Debug,PartialEq,Eq,Clone,StageLabel)]
pub enum DWayStage{
    Init,
    Desktop,
    Locked,
    Overview,
    Fullscreen,
    Moving,
    Resizing,
    Eixt,
}
