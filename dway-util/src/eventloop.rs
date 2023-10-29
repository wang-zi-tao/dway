use bevy::prelude::{error, Last, NonSend, NonSendMut, Plugin, Startup};
use calloop::channel::{Channel, Sender};
pub use calloop::{generic::Generic, EventSource, Interest, Mode, PostAction, Readiness};
use log::info;
use std::{os::fd::AsRawFd, sync::mpsc, time::{Duration, Instant, SystemTime}};
use winit::event_loop::EventLoopProxy;

pub type Register = Box<dyn FnOnce(&mut calloop::LoopHandle<'static, ()>) + Send + Sync>;

pub struct EventLoop {
    channel: Option<Channel<Register>>,
    sender: Sender<Register>,
    tx: Option<mpsc::Sender<()>>,
}

pub enum EventLoopControl {
    Continue,
    Stop,
}

pub struct EventLoopRunner(Channel<Register>);
impl EventLoopRunner {
    pub fn start_thread(
        self,
        eventloop: &mut EventLoop,
        mut callback: impl FnMut() -> EventLoopControl + Send + Sync + 'static,
    ) {
        let (tx, rx) = mpsc::channel();
        eventloop.tx.replace(tx);
        std::thread::Builder::new()
            .name("eventloop".to_string())
            .spawn(move || {
                self.run(Duration::from_secs(2), move || {
                    while let Ok(_) = rx.try_recv() {}
                    let r = callback();
                    let _ = rx.recv();
                    r
                });
            })
            .unwrap();
    }

    pub fn run(self, duration: Duration, mut callback: impl FnMut() -> EventLoopControl + 'static) {
        let mut event_loop = calloop::EventLoop::<()>::try_new().unwrap();
        let mut handle = event_loop.handle();
        let signal = event_loop.get_signal();
        let _ = event_loop.handle().insert_source(self.0, move |e, _, _| {
            match e {
                calloop::channel::Event::Msg(r) => {
                    r(&mut handle);
                }
                calloop::channel::Event::Closed => {
                    info!("stoping eventloop");
                    signal.stop();
                }
            };
        });
        let signal = event_loop.get_signal();
        let _ = event_loop.run(duration, &mut (), |_| {
            match callback() {
                EventLoopControl::Continue => {}
                EventLoopControl::Stop => signal.stop(),
            };
        });
        info!("eventloop stopped");
    }
}

impl EventLoop {
    pub fn new() -> Self {
        let (sender, channel) = calloop::channel::channel();
        Self {
            sender,
            channel: Some(channel),
            tx: None,
        }
    }

    pub fn add<F: AsRawFd + 'static + Send + Sync>(
        &mut self,
        source: Generic<F, std::io::Error>,
        mut callback: impl for<'l> FnMut(Readiness, &'l mut F) + 'static + Send + Sync,
    ) {
        let _ = self.sender.send(Box::new(move |handle| {
            if let Err(e) = handle.insert_source(source, move |event, metadata, _| {
                if event.error {
                    return Ok(PostAction::Remove);
                }
                callback(event, metadata);
                Ok(PostAction::Continue)
            }) {
                error!("failed to insert source: {e:?}")
            };
        }));
    }

    pub fn add_fd_to_read(&mut self, fd: &impl AsRawFd) {
        self.add(
            Generic::new(fd.as_raw_fd(), Interest::READ, Mode::Level),
            |_, _| {},
        );
    }

    pub fn start(&mut self, callback: impl FnMut() -> EventLoopControl + Send + Sync + 'static) {
        let runner = self.runner();
        runner.start_thread(self, callback);
    }

    pub fn runner(&mut self) -> EventLoopRunner {
        let channel = self.channel.take().unwrap();
        EventLoopRunner(channel)
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        info!("stop eventloop");
    }
}

fn launch_on_winit(winit_proxy: NonSend<EventLoopProxy<()>>, mut eventloop: NonSendMut<EventLoop>) {
    let winit_eventloop_proxy = winit_proxy.clone();
    eventloop.start(Box::new(move || {
        let _ = winit_eventloop_proxy.send_event(());
        EventLoopControl::Continue
    }))
}

fn on_frame_finish(eventloop: NonSendMut<EventLoop>) {
    if let Some(tx) = &eventloop.tx {
        let _ = tx.send(());
    }
}

#[derive(Default)]
pub enum EventLoopPlugin {
    #[default]
    WinitMode,
    ManualMode,
}
impl Plugin for EventLoopPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.insert_non_send_resource(EventLoop::new());
        match self {
            EventLoopPlugin::WinitMode => {
                app.add_systems(Startup, launch_on_winit);
            }
            EventLoopPlugin::ManualMode => {}
        }
        app.add_systems(Last, on_frame_finish);
    }
}
