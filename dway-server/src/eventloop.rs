use std::{io, os::unix::net::UnixStream, sync::Arc};

use bevy::prelude::*;
use calloop::{
    generic::Generic, EventLoop, EventSource, Interest, Mode, Poll, PostAction, Readiness, Token,
    TokenFactory,
};
use failure::Error;
use inlinable_string::InlinableString;
use wayland_server::ListeningSocket;

pub struct ListeningSocketEvent(pub Generic<ListeningSocket>);
impl EventSource for ListeningSocketEvent {
    type Event = UnixStream;
    type Metadata = ();
    type Ret = ();
    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> io::Result<PostAction>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.0.process_events(readiness, token, |_, socket| {
            while let Some(client) = socket.accept()? {
                debug!(socket = ?socket.socket_name(), client = ?client, "New client connected");
                callback(client, &mut ());
            }

            Ok(PostAction::Continue)
        })
    }

    fn register(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.0.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.0.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        self.0.unregister(poll)
    }
}
impl ListeningSocketEvent {
    pub fn new() -> Self {
        ListeningSocketEvent(Generic::new(
            ListeningSocket::bind_auto("wayland", 1..33).unwrap(),
            Interest::READ,
            Mode::Level,
        ))
    }
    pub fn filename(&self) -> InlinableString {
        InlinableString::from(&*self.0.file.socket_name().unwrap().to_string_lossy())
    }
}
