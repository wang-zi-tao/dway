use bevy::prelude::Resource;
use crossbeam_channel::{Receiver, Sender};

use dway_protocol::window::WindowMessage;


#[derive(Resource)]
pub struct WindowMessageReceiver(pub Receiver<WindowMessage>);

#[derive(Resource)]
pub struct WindowMessageSender(pub Sender<WindowMessage>);
