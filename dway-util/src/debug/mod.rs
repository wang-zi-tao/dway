use bevy::{prelude::*, remote::RemotePlugin, render::RenderApp};

pub struct RemoteDebugPlugin;
impl Plugin for RemoteDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RemotePlugin::default());
    }
}
