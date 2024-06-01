use bevy::prelude::*;
use tokio::runtime::{EnterGuard, Runtime};

pub struct TokioRuntime{
    runtime: Box<Runtime>,
}

impl std::ops::Deref for TokioRuntime {
    type Target = Runtime;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

#[derive(Default)]
pub struct TokioPlugin{}
impl Plugin for TokioPlugin{
    fn build(&self, app: &mut App) {
        let rt =Box::new(Runtime::new().unwrap());
        app.insert_non_send_resource(TokioRuntime{
            runtime: rt,
        });
    }
}
